use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Config {
	bb_path: Option<PathBuf>,
}

const CONFIG_FILE: &str = "config.toml";

fn find_bb() -> Result<PathBuf> {
	let steam_dir = steamlocate::SteamDir::locate().context("steamlocate couldn't locate Steam")?;
	match steam_dir.find_app(365360)? {
		Some((app, lib)) => Ok(lib.resolve_app_dir(&app)),
		None => Err(anyhow!("Couldn't locate Battle Brothers")),
	}
}

#[derive(Debug)]
pub struct DataPath(PathBuf);

impl DataPath {
	pub fn new(path: PathBuf) -> Self {
		DataPath(path)
	}

	pub fn join(&self, path: &str) -> PathBuf {
		self.0.join(path)
	}
}

impl AsRef<Path> for DataPath {
	fn as_ref(&self) -> &Path {
		&self.0
	}
}

#[derive(Debug)]
pub struct ExePath(PathBuf);

impl ExePath {
	pub fn new(path: PathBuf) -> Self {
		ExePath(path)
	}
}

impl AsRef<Path> for ExePath {
	fn as_ref(&self) -> &Path {
		&self.0
	}
}

impl Default for Config {
	fn default() -> Self {
		Self {
			// bb_path: None,
			bb_path: find_bb().ok(),
		}
	}
}

impl Config {
	pub fn load_or_default() -> Self {
		match Self::load() {
			Ok(config) => config,
			Err(_) => Config::default(),
		}
	}

	#[cfg(test)]
	pub fn from_path(path: PathBuf) -> Self {
		Self {
			bb_path: Some(path),
		}
	}

	pub fn save(&self) -> Result<()> {
		let config_text = toml::to_string(self).context("Couldn't serialize config file")?;
		std::fs::write(CONFIG_FILE, config_text).context("Couldn't write config file")?;
		Ok(())
	}

	fn load() -> Result<Self> {
		let config_text =
			std::fs::read_to_string(CONFIG_FILE).context("Couldn't read config file")?;
		let config: Config =
			toml::from_str(&config_text).context("Couldn't deserialize config file")?;
		Ok(config)
	}

	pub fn bb_path_known(&self) -> bool {
		self.bb_path.is_some()
	}

	// todo check that exe exists
	pub fn get_bb_exe_path(&self) -> Option<ExePath> {
		self.bb_path
			.as_ref()
			.map(|bb_path| ExePath::new(bb_path.join("win32").join("BattleBrothers.exe")))
			.filter(|exe_path| exe_path.as_ref().exists())
	}

	pub fn get_bb_data_path(&self) -> Option<DataPath> {
		self.bb_path
			.as_ref()
			.map(|bb_path| DataPath::new(bb_path.join("data")))
			.filter(|data_path| data_path.join("data_001.dat").exists())
	}

	pub fn set_path_from_exe<'a>(&'a mut self, exe_path: &'a Path) -> Result<&'a Path> {
		if exe_path.file_name().context("Couldn't get exe file name")? != "BattleBrothers.exe" {
			return Err(anyhow!("Not a Battle Brothers exe"));
		}
		let win32_dir = exe_path.parent().context("Couldn't get win32 dir")?;
		if win32_dir
			.file_name()
			.context("Couldn't get win32 dir name")?
			!= "win32"
		{
			return Err(anyhow!("Not a Battle Brothers win32 dir"));
		}
		let bb_dir = win32_dir.parent().context("Couldn't get bb dir")?;
		if !bb_dir.join("data").join("data_001.dat").exists() {
			return Err(anyhow!("Couldn't find valid data folder"));
		}
		self.bb_path = Some(bb_dir.to_path_buf());
		self.save()?;

		Ok(bb_dir)
	}
}
