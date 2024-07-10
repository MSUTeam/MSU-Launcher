use anyhow::{anyhow, Result};
use bytes::Bytes;
use dioxus::signals::{Readable, SyncSignal, Writable};
use sha2::{Digest, Sha256};
use std::{
	fs::File,
	io::{Cursor, Read, Write},
	path::Path,
};
use zip::ZipArchive;

use crate::config::Config;

const STEAMLESS_CLI: &str = "Steamless.CLI.exe";
const STEAMLESS_PLUGIN_FOLDER: &str = "Plugins";
const STEAMLESS_API_NAME: &str = "Steamless.API.dll";
const STEAMLESS_31_X86_VARIANT_NAME: &str = "Steamless.Unpacker.Variant31.x86.dll";

pub const ZIP_URL: &str = "https://github.com/atom0s/Steamless/releases/download/v3.1.0.5/Steamless.v3.1.0.5.-.by.atom0s.zip";
const STEAMLESS_HASH: [u8; 32] = match const_hex::const_decode_to_array(
	b"E3E2D22E098FF3FB359B2876AA2BED9596F0501E6FF588CBFFAE90A76D2DC4F5",
) {
	Ok(array) => array,
	Err(_) => unreachable!(),
};

fn extract_file_to_path(
	zip: &mut ZipArchive<Cursor<Bytes>>,
	zip_path: &Path,
	base_path: &Path,
) -> Result<()> {
	let path = base_path.join(zip_path);
	let mut zip_file = zip.by_name(&zip_path.to_string_lossy().replace('\\', "/"))?;
	let mut extracted_bytes = Vec::new();
	zip_file.read_to_end(&mut extracted_bytes)?;
	if let Some(parent) = path.parent() {
		std::fs::create_dir_all(parent)?;
	}
	let mut output_file = File::create(path)?;
	output_file.write_all(&extracted_bytes)?;
	Ok(())
}

async fn download_steamless(url: &str, target_path: &Path) -> Result<()> {
	let response = reqwest::get(url).await?.bytes().await?;
	let hash = <Sha256 as Digest>::digest(response.as_ref());
	if hash.as_slice() != STEAMLESS_HASH {
		return Err(anyhow!(
			"Hash mismatch for steamless (downloaded {} vs saved {}), erroring to prevent potential security risk"
		, const_hex::encode(hash), const_hex::encode(STEAMLESS_HASH)));
	}

	let reader = Cursor::new(response);
	let mut zip = zip::ZipArchive::new(reader)?;
	let plugins_folder = Path::new(STEAMLESS_PLUGIN_FOLDER);
	extract_file_to_path(&mut zip, Path::new(STEAMLESS_CLI), target_path)?;
	extract_file_to_path(
		&mut zip,
		&plugins_folder.join(STEAMLESS_API_NAME),
		target_path,
	)?;
	extract_file_to_path(
		&mut zip,
		&plugins_folder.join(STEAMLESS_31_X86_VARIANT_NAME),
		target_path,
	)?;
	Ok(())
}

async fn download_steamless_from_config(mut config: SyncSignal<Config>) -> Result<()> {
	let path = config.with(|c| c.get_steamless_path().to_owned());
	let result = download_steamless(ZIP_URL, &path).await;
	if let Err(e) = result {
		tracing::error!("Failed to download steamless: {}", e);
		Err(e)
	} else {
		let info = "Successfully installed steamless, ready to apply 4GB patch";
		config.with_mut(|c| {
			c.check_steamless_installed();
		});
		tracing::info!("{}", info);
		Ok(())
	}
}

pub async fn mt_download_steamless_from_config(config: SyncSignal<Config>) {
	let _ = tokio::spawn(async move {
		let _ = download_steamless_from_config(config).await;
	})
	.await;
}

pub fn is_steamless_installed(path: &Path) -> bool {
	let plugins_folder = path.join(STEAMLESS_PLUGIN_FOLDER);
	path.join(STEAMLESS_CLI).exists()
		&& plugins_folder.join(STEAMLESS_API_NAME).exists()
		&& plugins_folder.join(STEAMLESS_31_X86_VARIANT_NAME).exists()
}
