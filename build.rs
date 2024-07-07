use std::{
	collections::HashMap,
	fs::{self, File},
	io::Read,
	path::Path,
};

use anyhow::Result;
use dagrs::{Dag, DagError, DefaultTask, EnvVar, Input, Output, Task};
use image::codecs::ico::IcoFrame;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{
	digest::{generic_array::GenericArray, OutputSizeUser},
	Digest, Sha256,
};
use std::{
	fmt::Display,
	sync::{Arc, Mutex},
};

const CACHE_FILE: &str = "target/build_cache.ron";

const RAW_ASSETS: &str = "raw_assets";
const ASSETS: &str = "assets/assets";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NeedUpdate {
	Yes,
	No,
}

#[derive(thiserror::Error, Debug)]
pub enum CacheError {
	#[error("IO error: {0}")]
	IO(#[from] std::io::Error),
	#[error("RON Error: {0}")]
	Ron(#[from] ron::error::Error),
	#[error("RON Spanned error: {0}")]
	RonSpanned(#[from] ron::error::SpannedError),
	#[error("Error: {0}")]
	Other(String),
}

pub type CacheResult<T> = Result<T, CacheError>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Sha256Hash([u8; 32]);

impl From<GenericArray<u8, <Sha256 as OutputSizeUser>::OutputSize>> for Sha256Hash {
	fn from(array: GenericArray<u8, <Sha256 as OutputSizeUser>::OutputSize>) -> Self {
		let mut bytes = [0; 32];
		bytes.copy_from_slice(&array);
		Self(bytes)
	}
}

impl<'de> Deserialize<'de> for Sha256Hash {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let string = <String>::deserialize(deserializer)?;
		let mut bytes = [0; 32];
		hex::decode_to_slice(string, &mut bytes).map_err(serde::de::Error::custom)?;
		Ok(Self(bytes))
	}
}

impl Serialize for Sha256Hash {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serde::Serialize::serialize(&hex::encode(self.0), serializer)
	}
}

impl PartialEq<Option<Sha256Hash>> for Sha256Hash {
	fn eq(&self, other: &Option<Self>) -> bool {
		match other {
			Some(other) => self == other,
			None => false,
		}
	}
}

fn try_open(path: &Path) -> CacheResult<Option<File>> {
	match File::open(path) {
		Ok(file) => Ok(Some(file)),
		Err(e) => {
			if e.kind() == std::io::ErrorKind::NotFound {
				Ok(None)
			} else {
				Err(e.into())
			}
		}
	}
}

fn hash_file(file: &mut File) -> CacheResult<Sha256Hash> {
	// probably worth buffering very large files
	let mut file_contents = Vec::new();
	file.read_to_end(&mut file_contents)?;
	Ok(<Sha256 as Digest>::digest(file_contents).into())
}

#[derive(Default)]
pub struct Cache {
	cached_map: HashMap<Box<Path>, Sha256Hash>,
	current_map: HashMap<Box<Path>, Option<Sha256Hash>>,
}

#[cfg(test)]
impl PartialEq for Cache {
	fn eq(&self, other: &Self) -> bool {
		self.cached_map == other.cached_map
			&& self
				.current_map
				.read()
				.unwrap()
				.eq(&other.current_map.read().unwrap())
	}
}

impl Cache {
	fn from_cached_map(cached_map: HashMap<Box<Path>, Sha256Hash>) -> Self {
		Self {
			cached_map,
			current_map: HashMap::new(),
		}
	}

	pub fn is_modified(&mut self, path: &Path) -> CacheResult<bool> {
		if let Some(&cached) = self.cached_map.get(path) {
			if let Some(&current) = self.current_map.get(path) {
				Ok(cached != current)
			} else {
				let mut file = match try_open(path)? {
					Some(file) => file,
					None => {
						self.current_map.insert(path.into(), None);
						return Ok(true);
					}
				};
				let hash: Sha256Hash = hash_file(&mut file)?;
				self.current_map.insert(path.into(), Some(hash));
				Ok(cached != hash)
			}
		} else {
			self.current_map.insert(path.into(), None);
			Ok(true)
		}
	}
}

fn any_need_update<E>(inputs: &Input) -> Result<NeedUpdate, E>
where
	E: From<CacheError>,
{
	for input in inputs.get_iter() {
		if let Some(&updated) = input.get::<NeedUpdate>() {
			if updated == NeedUpdate::Yes {
				return Ok(NeedUpdate::Yes);
			}
		} else {
			return Err(CacheError::Other("Input was not a boolean".to_string()).into());
		}
	}
	Ok(NeedUpdate::No)
}

fn execute_if_needed<F, E>(inputs: &Input, env_var: Arc<EnvVar>, operation: &F) -> Output
where
	F: Fn(&Input, Arc<EnvVar>) -> Result<NeedUpdate, E>,
	E: From<CacheError> + Display,
{
	match any_need_update(inputs).and_then(|update| {
		if update == NeedUpdate::No {
			Ok(NeedUpdate::No)
		} else {
			operation(inputs, env_var)
		}
	}) {
		Ok(need_update) => Output::new(need_update),
		Err(e) => Output::error(e.to_string()),
	}
}

#[derive(Default)]
pub struct TasksCache {
	cache: Arc<Mutex<Cache>>,
	exists_tasks: HashMap<Box<Path>, usize>,
	modified_tasks: HashMap<Box<Path>, usize>,
	tasks: Vec<DefaultTask>,
}

impl TasksCache {
	fn from_cached_map(cached_map: HashMap<Box<Path>, Sha256Hash>) -> Self {
		Self {
			cache: Arc::new(Mutex::new(Cache::from_cached_map(cached_map))),
			exists_tasks: HashMap::new(),
			modified_tasks: HashMap::new(),
			tasks: Vec::new(),
		}
	}

	pub fn get_exists_task<T>(&mut self, path: T) -> usize
	where
		T: Into<Box<Path>>,
	{
		let path = path.into();
		if let Some(&task_id) = self.exists_tasks.get(&path) {
			task_id
		} else {
			let unmoved_path = path.clone();
			let task = dagrs::DefaultTask::with_closure(
				&format!("Exists|{}", path.display()),
				move |_, _| {
					Output::new({
						match path.exists() {
							true => NeedUpdate::No,
							false => NeedUpdate::Yes,
						}
					})
				},
			);
			let id = task.id();
			self.tasks.push(task);
			self.exists_tasks.insert(unmoved_path, id);
			id
		}
	}

	pub fn get_modified_task<T>(&mut self, path: T) -> usize
	where
		T: Into<Box<Path>>,
	{
		let path = path.into();
		if let Some(&task_id) = self.modified_tasks.get(&path) {
			task_id
		} else {
			let unmoved_path = path.clone();
			let cache = self.cache.clone();
			let task = dagrs::DefaultTask::with_closure(
				&format!("Modified|{}", path.display()),
				move |_, _| match cache.lock().unwrap().is_modified(&path) {
					Ok(modified) => Output::new(match modified {
						true => NeedUpdate::Yes,
						false => NeedUpdate::No,
					}),
					Err(e) => Output::error(e.to_string()),
				},
			);
			let id = task.id();
			self.tasks.push(task);
			self.modified_tasks.insert(unmoved_path, id);
			id
		}
	}

	pub fn make_task<F, I1, I2, E>(
		&mut self,
		name: &str,
		inputs: I1,
		outputs: I2,
		closure: F,
	) -> usize
	where
		F: Fn(&Input, Arc<EnvVar>) -> Result<NeedUpdate, E> + Send + Sync + 'static,
		I1: IntoIterator<Item: AsRef<Path>>,
		I2: IntoIterator<Item: AsRef<Path>>,
		E: From<CacheError> + Display,
	{
		let mut task = DefaultTask::with_closure(name, move |inputs, env_var| {
			execute_if_needed(&inputs, env_var, &closure)
		});
		task.set_predecessors_by_id(
			inputs
				.into_iter()
				.map(|path| self.get_modified_task(path.as_ref())),
		);
		task.set_predecessors_by_id(
			outputs
				.into_iter()
				.map(|path| self.get_exists_task(path.as_ref())),
		);
		let id = task.id();
		self.tasks.push(task);
		id
	}

	pub fn run_with_tasks<I>(&mut self, tasks: I) -> Result<bool, DagError>
	where
		I: IntoIterator<Item = DefaultTask>,
	{
		let tasks = self.tasks.drain(..).chain(tasks);
		let mut dag = Dag::with_tasks(tasks.collect());
		dag.start()
	}
}

pub fn load_cache() -> CacheResult<TasksCache> {
	if let Some(file) = try_open(CACHE_FILE.as_ref())? {
		Ok(TasksCache::from_cached_map(ron::de::from_reader(file)?))
	} else {
		Ok(TasksCache::default())
	}
}

pub fn save_cache(tasks_cache: &TasksCache) -> CacheResult<()> {
	let writer = File::create(CACHE_FILE)?;
	let cache = tasks_cache.cache.lock().unwrap();
	let mut merged_map = cache.cached_map.clone();
	{
		for (path, hash) in cache.current_map.iter() {
			if let Some(hash) = hash {
				merged_map.insert(path.clone(), *hash);
			} else if let Ok(mut file) = File::open(path) {
				if let Ok(hash) = hash_file(&mut file) {
					merged_map.insert(path.clone(), hash);
				}
			}
		}
	}
	ron::ser::to_writer_pretty(
		writer,
		&merged_map,
		PrettyConfig::default()
			.indentor("\t".to_string())
			.new_line("\n".to_string()),
	)?;
	Ok(())
}

pub fn clear_cache() -> CacheResult<bool> {
	fs::remove_file(CACHE_FILE).map(|_| true).or_else(|e| {
		if e.kind() == std::io::ErrorKind::NotFound {
			Ok(false)
		} else {
			Err(e.into())
		}
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_serialization() {
		let mut cache = Cache {
			cached_map: HashMap::new(),
			current_map: RwLock::new(HashMap::new()),
		};
		cache
			.cached_map
			.insert(Path::new("test").into(), Sha256Hash([0; 32]));
		let serialized = ron::ser::to_string(&cache).unwrap();
		let new_cache: Cache = ron::de::from_str(&serialized).unwrap();
		assert_eq!(cache, new_cache);
		println!("{}", serialized);
	}

	// #[test]
	// fn mock() {
	// 	let
	// }

	#[test]
	fn test_save_load() {
		let mut cache = Cache::default();
		cache
			.cached_map
			.insert(Path::new("test").into(), Sha256Hash([0; 32]));
		save_cache(&cache).unwrap();
		let new_cache = load_cache().unwrap();
		assert_eq!(&cache, &new_cache);

		cache
			.current_map
			.write()
			.unwrap()
			.insert(Path::new("test").into(), Some(Sha256Hash([1; 32])));
		save_cache(&cache).unwrap();
		let new_cache = load_cache().unwrap();

		let new_expected = Cache {
			cached_map: {
				let mut map = HashMap::new();
				map.insert(Path::new("test").into(), Sha256Hash([1; 32]));
				map
			},
			current_map: RwLock::new(HashMap::new()),
		};
		assert_eq!(new_expected, new_cache);
	}
}

fn create_ico_from_png(png_path: &str, ico_path: &str) -> Result<()> {
	// println!("cargo::rerun-if-changed=assets_raw/msu_logo_full.png");
	let img = image::open(png_path)?;
	let sizes = [16, 24, 32, 48, 64, 128, 256];
	let out_file = File::create(ico_path)?;
	let ico_encoder = image::codecs::ico::IcoEncoder::new(out_file);
	let images = sizes.iter().map(|size| {
		let resized = img.resize(*size, *size, image::imageops::FilterType::CatmullRom);
		// afaik this shouldn't be necessary since the resized image should already be in png format since the original is too
		// but it breaks if I don't do it
		IcoFrame::as_png(
			&resized.into_bytes(),
			*size,
			*size,
			image::ExtendedColorType::Rgb8,
		)
		.map_err(Into::<anyhow::Error>::into)
	});
	let images = images.collect::<Result<Vec<_>>>()?;
	ico_encoder.encode_images(&images)?;
	Ok(())
}

fn create_icon(_: &Input, _: Arc<EnvVar>) -> Result<NeedUpdate> {
	let png_string = format!("{}/msu_logo.png", RAW_ASSETS);
	let ico_string = format!("{}/gfx/icons/msu_logo.ico", ASSETS);
	create_ico_from_png(&png_string, &ico_string)?;
	Ok(NeedUpdate::Yes)
}

fn attach_icon_to_exe(_: Input, _: Arc<EnvVar>) -> Output {
	let ico_string = format!("{}/gfx/icons/msu_logo.ico", ASSETS);
	match winresource::WindowsResource::new()
		.set_icon(&ico_string)
		.compile()
	{
		Ok(_) => Output::empty(),
		Err(e) => Output::error(e.to_string()),
	}
}

fn main() -> Result<()> {
	let mut cache = load_cache()?;

	let create_icon_id = cache.make_task(
		"CreateIcon",
		[&format!("{}/msu_logo.png", RAW_ASSETS)],
		[&format!("{}/gfx/icons/msu_logo.ico", ASSETS)],
		create_icon,
	);

	let mut attach_icon = DefaultTask::with_closure("AttachIconToExe", attach_icon_to_exe);
	attach_icon.set_predecessors_by_id([create_icon_id]);

	cache.run_with_tasks([attach_icon])?;

	save_cache(&cache)?;
	Ok(())
}
