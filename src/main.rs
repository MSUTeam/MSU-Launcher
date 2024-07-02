#![cfg_attr(feature = "bundle", windows_subsystem = "windows")]

use crate::button::{
	LaunchButton, Run4GBPatcherButton, RunPreloadPatcherButton, SetGameLocationButton,
};
use crate::log::InfoPanel;
use anyhow::Result;
use button::DonateButton;
use config::Config;
use dioxus::desktop::tao::platform::windows::{IconExtWindows, WindowBuilderExtWindows};
use dioxus::desktop::LogicalSize;
use dioxus::{
	desktop::{
		tao::{dpi::Size, window::Icon},
		WindowBuilder,
	},
	prelude::*,
};
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

#[cfg(feature = "bundle")]
const ASSETS: &str = "assets";
#[cfg(not(feature = "bundle"))]
const ASSETS: &str = "assets/assets";

fn build_window() -> WindowBuilder {
	WindowBuilder::new()
		.with_maximizable(false)
		.with_resizable(false)
		.with_inner_size(Size::Logical(LogicalSize {
			width: 1024.0,
			height: 768.0,
		}))
		.with_title("MSU Launcher")
		.with_window_icon(
			Icon::from_path(
				format!("{}/gfx/icons/msu_logo.ico", ASSETS),
				Some([16, 16].into()),
			)
			.ok(),
		)
		.with_taskbar_icon(
			Icon::from_path(
				format!("{}/gfx/icons/msu_logo.ico", ASSETS),
				Some([32, 32].into()),
			)
			.ok(),
		)
}

fn main() {
	// Init logger
	once_cell::sync::Lazy::force(&log::TRACING);
	tracing::info!("Starting MSU Launcher");
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
		div {
			class: "w-full flex justify-center items-center relative",
			style,
			DonateButton { class: "left-3 top-3 absolute" }
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
fn ButtonBar() -> Element {
	let config = use_signal_sync(Config::load_or_default);
	rsx!(
		div { class: "flex h-fit justify-between items-center space-x-2 w-[90%]",
			SetGameLocationButton { class: "p-1 text-xl normal-font", config }
			LaunchButton { class: "flex-grow h-full text-4xl title-font", config }
			div { class: "flex flex-col space-y-1",
				RunPreloadPatcherButton { class: "p-1 h-1/2 text-xl normal-font", config }
				Run4GBPatcherButton { class: "p-1 h-1/2 text-xl normal-font", config }
			}
		}
	)
}

#[component]
fn Content(style: Option<String>) -> Element {
	let style = style.unwrap_or_default();
	rsx!(
		div {
			class: "flex flex-col h-full w-full justify-center items-center",
			style,
			Center {}
			InfoPanel { class: "w-[90%] h-12 mb-4" }
			ButtonBar {}
		}
	)
}

#[component]
fn App() -> Element {
	rsx! {
		Header { style: "height: 10.4%;" }
		Content { style: "height: 89.6%;" }
	}
}
