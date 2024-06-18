#![cfg_attr(feature = "bundle", windows_subsystem = "windows")]

use crate::button::{
	LaunchButton, Run4GBPatcherButton, RunPreloadPatcherButton, SetGameLocationButton,
};
use crate::log::{InfoLog, InfoPanel};
use anyhow::Result;
use config::Config;
use dioxus::desktop::tao::platform::windows::WindowBuilderExtWindows;
use dioxus::desktop::LogicalSize;
use dioxus::{
	desktop::{
		tao::{dpi::Size, window::Icon},
		WindowBuilder,
	},
	prelude::*,
};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tracing::Level;
mod button;
mod config;
mod log;
mod patcher_laa;
mod patcher_preload;
mod steamless;

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
	#[route("/")]
	App {},
}

// I shouldn't have to do this but this is the only cross-platform way of doing this I could find
fn load_png_icon(png_path: &Path) -> Result<Icon> {
	let mut image_reader = image::io::Reader::new(BufReader::new(File::open(png_path)?));
	image_reader.set_format(image::ImageFormat::Png);
	let rgba_image = image_reader.decode()?.into_rgba8();
	let rgba_bytes = rgba_image.into_raw();
	let dimensions = ((rgba_bytes.len() / 4) as f64).sqrt() as u32;
	let icon = Icon::from_rgba(rgba_bytes, dimensions, dimensions)?;
	Ok(icon)
}

fn build_window_raw() -> WindowBuilder {
	WindowBuilder::new()
		.with_maximizable(false)
		.with_resizable(false)
		.with_inner_size(Size::Logical(LogicalSize {
			width: 1024.0,
			height: 768.0,
		}))
		.with_title("MSU Launcher")
}

#[cfg(feature = "bundle")]
fn build_window() -> WindowBuilder {
	build_window_raw()
		.with_window_icon(Some(
			load_png_icon(Path::new("./assets/gfx/icons/icon16x16.png")).unwrap(),
		))
		.with_taskbar_icon(Some(
			load_png_icon(Path::new("./assets/gfx/icons/icon32x32.png")).unwrap(),
		))
}

#[cfg(not(feature = "bundle"))]
fn build_window() -> WindowBuilder {
	build_window_raw()
		.with_window_icon(Some(
			load_png_icon(Path::new("./assets/assets/gfx/icons/icon16x16.png")).unwrap(),
		))
		.with_taskbar_icon(Some(
			load_png_icon(Path::new("./assets/assets/gfx/icons/icon32x32.png")).unwrap(),
		))
}

fn main() {
	// Init logger
	dioxus_logger::init(Level::INFO).expect("failed to init logger");
	let cfg = dioxus::desktop::Config::new()
		.with_custom_head(
			r#"
		<link rel="stylesheet" href="assets/style/tailwind.css">
		<link rel="stylesheet" href="assets/main.css">
		"#
			.to_string(),
		)
		.with_window(build_window());
	LaunchBuilder::desktop().with_cfg(cfg).launch(App);
}

#[component]
fn Header(style: Option<String>) -> Element {
	let style = style.unwrap_or_default();
	rsx! {
		div { class: "w-full flex justify-center items-center", style,
			h1 { class: "title-font text-6xl", "MSU Launcher" }
		}
	}
}

#[component]
fn Center() -> Element {
	rsx!(
		div { class: "h-4/6 w-full flex flex-col justify-center items-center",
			p { "Mod List Manager? Conflict Analyzer? Mod Update Checker?" }
		}
	)
}

#[component]
fn ButtonBar(logger: SyncSignal<InfoLog>) -> Element {
	let config = use_signal_sync(Config::load_or_default);
	rsx!(
		div { class: "flex h-fit justify-between items-center space-x-2 w-[90%]",
			SetGameLocationButton { class: "p-1 text-xl normal-font", config, logger }
			LaunchButton {
				class: "flex-grow h-full text-4xl title-font",
				config,
				logger
			}
			div { class: "flex flex-col space-y-1",
				RunPreloadPatcherButton { class: "p-1 h-1/2 text-xl normal-font", config, logger }
				Run4GBPatcherButton { class: "p-1 h-1/2 text-xl normal-font", config, logger }
			}
		}
	)
}

#[component]
fn Content(style: Option<String>, logger: SyncSignal<InfoLog>) -> Element {
	let style = style.unwrap_or_default();
	rsx!(
		div {
			class: "flex flex-col h-full w-full justify-center items-center",
			style,
			Center {}
			InfoPanel { class: "w-[90%] h-12 mb-4", logger }
			ButtonBar { logger }
		}
	)
}

#[component]
fn App() -> Element {
	let logger = use_signal_sync(|| InfoLog::new(100));
	rsx! {
		Header { style: "height: 15.7%;" }
		Content { style: "height: 84.3%;", logger }
	}
}
