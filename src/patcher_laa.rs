use crate::config::Config;
use anyhow::{anyhow, Context, Result};
use dioxus::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io::{Read, Seek, SeekFrom, Write};
use std::mem::size_of;
use std::process::Command;
use std::{fs::File, path::Path};
use windows::Win32::System::Diagnostics::Debug::{
	IMAGE_FILE_CHARACTERISTICS, IMAGE_FILE_HEADER, IMAGE_FILE_LARGE_ADDRESS_AWARE,
};
use windows::Win32::System::SystemServices::IMAGE_DOS_HEADER;

// I'm not the biggest fan of this approach
// but I don't have an alternative reliable way of differentiating
// between steam, gog and steamless versions of the game
const GOG_HASH_STR: &str = include_str!("../hashes/gog.txt");
const STEAM_HASH_STR: &str = include_str!("../hashes/steam.txt");
const STEAMLESS_HASH_STR: &str = include_str!("../hashes/steamless.txt");

fn get_hash_set_from_str(hash_str: &str) -> HashSet<Vec<u8>> {
	hash_str
		.lines()
		.map(|line| const_hex::decode(line).unwrap())
		.collect()
}

fn remove_steam_drm(original_path: &Path) -> Result<()> {
	// bad approach, want to improve this by using the steamless API dlls
	// or ideally dll injection as suggested by MonochromeWench
	let out = Command::new("./steamless/Steamless.CLI.exe")
		.arg(original_path)
		.output()?;
	match out.status.code() {
		Some(0) => Ok(()),
		Some(code) => Err(anyhow!("Steamless failed with code {}", code)),
		None => Err(anyhow!("Steamless failed with no code")),
	}?;
	let new_path_str = format!(
		"{}.unpacked.exe",
		original_path
			.to_str()
			.context("Failed to convert original_file path to string")?
	);
	let new_str = Path::new(&new_path_str);
	if !new_str.exists() {
		return Err(anyhow!("Steamless didn't create a new file"));
	}

	std::fs::rename(new_str, original_path)?;
	Ok(())
}

fn read_and_check_pe_magic_number(file: &mut File, seek_back: bool) -> Result<()> {
	let mut pe_magic_number: [u8; 4] = [0; 4];
	file.read_exact(&mut pe_magic_number)?;

	if pe_magic_number != [0x50, 0x45, 0, 0] {
		return Err(anyhow!("Invalid PE magic number"));
	}

	if seek_back {
		file.seek(SeekFrom::Current(-(size_of::<[u8; 4]>() as i64)))?;
	}

	Ok(())
}

fn seek_to_pe_header(file: &mut File) -> Result<()> {
	file.seek(SeekFrom::Start(0))?;
	let mut dos_header = IMAGE_DOS_HEADER::default();
	file.read_exact(unsafe {
		std::slice::from_raw_parts_mut(
			std::ptr::from_mut(&mut dos_header) as *mut u8,
			size_of::<IMAGE_DOS_HEADER>(),
		)
	})?;

	if dos_header.e_magic != 0x5A4D {
		return Err(anyhow!(
			"Invalid DOS magic number : {:X}",
			dos_header.e_magic
		));
	}

	file.seek(SeekFrom::Start(dos_header.e_lfanew as u64))?;

	read_and_check_pe_magic_number(file, true)
}

fn read_image_file_header(file: &mut File) -> Result<IMAGE_FILE_HEADER> {
	read_and_check_pe_magic_number(file, false)?;
	let mut file_header = IMAGE_FILE_HEADER::default();
	file.read_exact(unsafe {
		std::slice::from_raw_parts_mut(
			std::ptr::from_mut(&mut file_header) as *mut u8,
			size_of::<IMAGE_FILE_HEADER>(),
		)
	})?;
	Ok(file_header)
}

fn write_image_file_header(file: &mut File, header: &IMAGE_FILE_HEADER) -> Result<()> {
	if file.metadata()?.permissions().readonly() {
		return Err(anyhow!(
			"Couldn't write IMAGE_FILE_HEADER: File is readonly"
		));
	}
	read_and_check_pe_magic_number(file, false)?;
	file.write(unsafe {
		core::slice::from_raw_parts(
			header as *const IMAGE_FILE_HEADER as *const u8,
			size_of::<IMAGE_FILE_HEADER>(),
		)
	})
	.context("Couldn't write IMAGE_FILE_HEADER")?;
	Ok(())
}

fn make_laa(path: &Path) -> Result<()> {
	let mut file = File::options().read(true).write(true).open(path)?;
	seek_to_pe_header(&mut file)?;
	let mut file_header = read_image_file_header(&mut file)?;
	file_header.Characteristics |= IMAGE_FILE_LARGE_ADDRESS_AWARE;
	seek_to_pe_header(&mut file)?;
	write_image_file_header(&mut file, &file_header)?;
	Ok(())
}

pub fn is_laa(path: &Path) -> Result<bool> {
	let mut file = File::open(path)?;
	seek_to_pe_header(&mut file)?;
	let file_header = read_image_file_header(&mut file)?;
	Ok(file_header.Characteristics & IMAGE_FILE_LARGE_ADDRESS_AWARE
		!= IMAGE_FILE_CHARACTERISTICS(0))
}

fn sha_hash_path(path: &Path) -> Result<Vec<u8>> {
	let mut file = File::open(path)?;
	let mut hasher = Sha256::new();
	std::io::copy(&mut file, &mut hasher)?;
	Ok(hasher.finalize().to_vec())
}

fn make_backup(path: &Path, backup_extension: &str) -> Result<()> {
	let backup_path = format!(
		"{}.{}",
		path.to_str()
			.with_context(|| format!("Couldn't parse file path {:?}", path))?,
		backup_extension
	);
	std::fs::copy(path, backup_path).with_context(move || {
		format!(
			"Failed to create backup of file {:?} with extension {}",
			path, backup_extension
		)
	})?;
	Ok(())
}

pub fn patch_exe(exe_path: &Path) -> Result<String> {
	let hash = sha_hash_path(exe_path)?;
	if get_hash_set_from_str(STEAM_HASH_STR).contains(&hash) {
		make_backup(exe_path, "steam_backup")?;
		remove_steam_drm(exe_path).context("Failed to remove Steam DRM")?;
		make_backup(exe_path, "steamless_backup")?;
		make_laa(exe_path).context("Failed to apply 4GB Patch")?;
		Ok("Patched Steam Version".to_string())
	} else if get_hash_set_from_str(STEAMLESS_HASH_STR).contains(&hash) {
		make_backup(exe_path, "steamless_backup")?;
		make_laa(exe_path).context("Failed to apply 4GB Patch")?;
		Ok("Patched Steamless Version".to_string())
	} else if get_hash_set_from_str(GOG_HASH_STR).contains(&hash) {
		make_backup(exe_path, "gog_backup")?;
		make_laa(exe_path).context("Failed to apply 4GB Patch")?;
		Ok("Patched GOG Version".to_string())
	} else if is_laa(exe_path)? {
		Ok("Already patched".to_string())
	} else {
		Err(anyhow!("Unknown version of Battle Brothers, verify or reinstall your game from a legitimate source"))
	}
}

pub fn patch_from_config(config: ReadOnlySignal<Config, SyncStorage>) -> Result<()> {
	let exe_path = match config.read().get_bb_exe_path() {
		Some(path) => path,
		None => {
			let error = "Couldn't find BattleBrothers.exe";
			tracing::error!("{}", error);
			return Err(anyhow!(error));
		}
	};
	match patch_exe(exe_path.as_ref()) {
		Ok(msg) => {
			tracing::info!("{}", msg);
			Ok(())
		}
		Err(e) => {
			tracing::error!("{}", e.to_string());
			Err(e)
		}
	}
}
