use anyhow::{anyhow, Result};
use dioxus::signals::{ReadOnlySignal, Readable, SyncSignal, SyncStorage, Writable};
use std::collections::HashSet;
use std::io::Write;
use std::{fs::File, io::Read, path::Path};
use zip::ZipArchive;
use zip::{write::SimpleFileOptions, CompressionMethod};

use crate::config::{Config, DataPath};
use crate::log::InfoLog;

const TABBED_NEWLINE: &str = "\n\t\t\t";

const ON_RUNNING_PATH: &str = "preload/on_running.txt";
const ON_START_PATH: &str = "preload/on_start.txt";

const MOD_ID: &str = "mod_load_patcher";
const ZIP_NAME: &str = "~mod_load_patcher.zip";
const MOD_NAME: &str = "Load Patcher";
const MOD_NAMESPACE: &str = "LoadPatcher";
const MOD_STRING: &str = include_str!("../squirrel/mod_resource_loader.nut");
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct ResourceGatherer {
	pub on_running: HashSet<String>,
	pub on_start: HashSet<String>,
}

impl ResourceGatherer {
	pub fn new() -> Self {
		Self {
			on_running: HashSet::new(),
			on_start: HashSet::new(),
		}
	}
}

pub struct ResourceHandler {
	on_running: Vec<String>,
	on_start: Vec<String>,
}

impl From<ResourceGatherer> for ResourceHandler {
	fn from(value: ResourceGatherer) -> Self {
		let mut on_running: Vec<_> = value.on_running.into_iter().collect();
		on_running.sort();
		let mut on_start: Vec<_> = value.on_start.into_iter().collect();
		on_start.sort();
		Self {
			on_running,
			on_start,
		}
	}
}

impl ResourceHandler {
	fn make_quoted_strings(strings: &[String]) -> String {
		let mut s = "[".to_owned();
		if !strings.is_empty() {
			s.push_str(TABBED_NEWLINE);
			for line in strings.iter() {
				s.push_str(&format!("\"{}\",{}", line, TABBED_NEWLINE));
			}
			s.replace_range(
				s.len() - TABBED_NEWLINE.len()..s.len(),
				&TABBED_NEWLINE[0..TABBED_NEWLINE.len() - 1],
			);
		}
		s.push(']');
		s
	}

	pub fn get_on_running_quoted(&self) -> String {
		ResourceHandler::make_quoted_strings(&self.on_running)
	}

	pub fn get_on_start_quoted(&self) -> String {
		ResourceHandler::make_quoted_strings(&self.on_start)
	}

	fn make_raw_strings(strings: &[String]) -> String {
		let mut s = String::new();
		for line in strings.iter() {
			s.push_str(&format!("{}\n", line));
		}
		s
	}

	pub fn get_on_running_raw(&self) -> String {
		ResourceHandler::make_raw_strings(&self.on_running)
	}

	pub fn get_on_start_raw(&self) -> String {
		ResourceHandler::make_raw_strings(&self.on_start)
	}
}

fn read_file_in_zip(zip_file: &mut ZipArchive<File>, name: &str) -> Result<String> {
	let mut file = match zip_file.by_name(name) {
		Err(zip::result::ZipError::FileNotFound) => return Ok(String::new()),
		Err(e) => return Err(anyhow!(e)),
		Ok(file) => file,
	};
	let mut contents = String::with_capacity(file.size() as usize);
	file.read_to_string(&mut contents)?;
	Ok(contents)
}

pub fn gather_resources_for_mod(gatherer: &mut ResourceGatherer, mod_path: &Path) -> Result<()> {
	let file = std::fs::File::open(mod_path)?;
	// not sure why the API requires this to be mut
	let mut zip_file = match zip::ZipArchive::new(file) {
		Err(zip::result::ZipError::InvalidArchive(_)) => return Ok(()),
		Err(e) => return Err(anyhow!(e)),
		Ok(zip) => zip,
	};
	for line in read_file_in_zip(&mut zip_file, ON_RUNNING_PATH)?.lines() {
		gatherer.on_running.insert(line.to_owned());
	}
	for line in read_file_in_zip(&mut zip_file, ON_START_PATH)?.lines() {
		gatherer.on_start.insert(line.to_owned());
	}
	Ok(())
}

pub fn get_resource_handler(data_path: &DataPath) -> Result<ResourceHandler> {
	let entries: Result<Vec<_>, _> = std::fs::read_dir(data_path)?.collect();
	let entries = entries?;
	let mut gatherer = ResourceGatherer::new();
	for e in entries.into_iter() {
		if let Ok(file_type) = e.file_type() {
			if file_type.is_dir() || e.file_name().to_string_lossy().ends_with(ZIP_NAME) {
				continue;
			}
			gather_resources_for_mod(&mut gatherer, &e.path())?;
		}
	}
	Ok(gatherer.into())
}

fn get_mod_string(resource_handler: &ResourceHandler) -> String {
	let mod_string = MOD_STRING.to_owned();
	let mod_string = mod_string.replace("$OnRunning$", &resource_handler.get_on_running_quoted());
	let mod_string = mod_string.replace("$OnStart$", &resource_handler.get_on_start_quoted());
	let mod_string = mod_string.replace("$Version$", &format!("\"{}\"", VERSION));
	let mod_string = mod_string.replace("$Name$", &format!("\"{}\"", MOD_NAME));
	let mod_string = mod_string.replace("$ID$", &format!("\"{}\"", MOD_ID));

	mod_string.replace("$NameSpace$", MOD_NAMESPACE)
}

pub fn create_mod(data_path: &DataPath, resources: &ResourceHandler) -> Result<()> {
	let mut zip = zip::ZipWriter::new(std::fs::File::create(data_path.join(ZIP_NAME))?);
	let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
	let mod_string = get_mod_string(resources);
	zip.start_file(format!("scripts/!mods_preload/{}.nut", MOD_ID), options)?;
	zip.write_all(mod_string.as_bytes())?;

	zip.start_file(ON_RUNNING_PATH, options)?;
	zip.write_all(resources.get_on_running_raw().as_bytes())?;
	zip.start_file(ON_START_PATH, options)?;
	zip.write_all(resources.get_on_start_raw().as_bytes())?;

	zip.finish()?;
	Ok(())
}

pub fn sync_gather_and_create_mod(data_path: &DataPath) -> Result<()> {
	let resources = get_resource_handler(data_path)?;
	create_mod(data_path, &resources)
}

pub async fn async_gather_and_create_mod(
	config: ReadOnlySignal<Config, SyncStorage>,
	mut logger: SyncSignal<InfoLog>,
) {
	let data_path = match config.read().get_bb_data_path() {
		Some(path) => path,
		None => {
			logger.with_mut(|l| {
				l.error("Couldn't find /data folder");
			});
			return;
		}
	};
	match sync_gather_and_create_mod(&data_path) {
		Ok(_) => {
			logger.with_mut(|l| {
				l.info("Patcher Succeeded");
			});
		}
		Err(e) => {
			logger.with_mut(|l| {
				l.error(format!("Couldn't create patcher : {}", e));
			});
		}
	}
}

pub async fn mt_gather_and_create_mod(
	config: ReadOnlySignal<Config, SyncStorage>,
	logger: SyncSignal<InfoLog>,
) {
	let _ = tokio::spawn(async move { async_gather_and_create_mod(config, logger).await }).await;
}
