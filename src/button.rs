use dioxus::prelude::*;
use std::path::PathBuf;

use crate::{patcher_laa, patcher_preload, steamless, Config};

#[component]
pub fn Button(
	onclick: EventHandler<MouseEvent>,
	class: Option<String>,
	style: Option<String>,
	id: Option<String>,
	#[props(default=ReadOnlySignal::default())] disabled: ReadOnlySignal<bool>,
	children: Element,
) -> Element {
	let class = class.unwrap_or_default();
	rsx!(
		button {
			class: "{class} msu-button",
			style,
			id,
			disabled,
			onclick: move |e| onclick.call(e),
			{children}
		}
	)
}

// This is necessary to go from Memo<bool> to ReadOnlySignal<bool> to then go to Option<ReadOnlySignal<bool>>
#[component]
pub fn DisableButton(
	onclick: EventHandler<MouseEvent>,
	class: Option<String>,
	style: Option<String>,
	id: Option<String>,
	disabled: ReadOnlySignal<bool>,
	children: Element,
) -> Element {
	rsx!(
		Button { class, style, id, disabled, onclick, {children} }
	)
}

#[component]
pub fn ConfigButton(
	class: Option<String>,
	style: Option<String>,
	config: SyncSignal<Config>,
) -> Element {
	let mut counter: i32 = 0;
	rsx!(
		Button {
			class,
			style,
			onclick: move |_| {
				println!("Config");
				tracing::info!("Config! {}", counter);
				counter += 1;
				tracing::error!("Config! {}", counter);
			},
			"Config"
		}
	)
}

async fn launch_game(config: ReadOnlySignal<Config, SyncStorage>) {
	patcher_preload::async_gather_and_create_mod(config).await;
	match config.read().launch_game() {
		Ok(_) => tracing::info!("Launched Battle Brothers"),
		Err(e) => tracing::error!("Couldn't launch Battle Brothers: {}", e),
	};
}

#[component]
pub fn DonateButton(
	#[props(default="".to_string())] class: String,
	style: Option<String>,
) -> Element {
	rsx!(
		a { href: "https://ko-fi.com/enduriel",
			div { class: "rounded-lg w-40 h-16 bg-gray-800 {class}", style,
				img {
					class: "left-0 top-[50%] w-[40%] h-[80%] absolute",
					style: "transform: translateY(-50%);",
					src: "assets/gfx/icons/kofi.svg"
				}
				div { class: "absolute top-1 w-[50%] h-[100%] right-3",
					div { class: "text-[12px] text-gray-300", "Support me on" }
					div { class: "text-3xl font-bold text-white", "Ko-fi" }
				}
			}
		}
	)
}

#[component]
pub fn LaunchButton(
	class: Option<String>,
	style: Option<String>,
	config: ReadOnlySignal<Config, SyncStorage>,
) -> Element {
	rsx!(
		Button {
			class,
			style,
			disabled: use_memo(move || !config.read().bb_path_known()),
			onclick: move |_| {
				spawn(async move {
					let _ = tokio::spawn(async move {
							launch_game(config).await;
						})
						.await;
				});
			},
			"Launch Battle Brothers"
		}
	)
}

#[component]
pub fn RunPreloadPatcherButton(
	class: Option<String>,
	style: Option<String>,
	config: ReadOnlySignal<Config, SyncStorage>,
) -> Element {
	rsx!(
		Button {
			class,
			style,
			disabled: use_memo(move || !config.read().bb_path_known()),
			onclick: move |_| {
				spawn(async move { patcher_preload::mt_gather_and_create_mod(config).await });
			},
			"Run Preload Patcher"
		}
	)
}

#[component]
pub fn Run4GBPatcherButton(
	class: Option<String>,
	style: Option<String>,
	config: SyncSignal<Config>,
) -> Element {
	config.with_mut(|c| c.check_steamless_installed());
	rsx!(
		Button {
			class,
			style,
			disabled: use_memo(move || !config.read().bb_path_known()),
			onclick: move |_| {
				spawn(async move {
					let steamless_installed = config
						.with_mut(|c| { c.check_steamless_installed() });
					if steamless_installed {
						let _ = patcher_laa::patch_from_config(config.into());
					} else {
						let _ = steamless::mt_download_steamless_from_config(config).await;
					}
				});
			},
			{
				use_memo(move || {
					if config.read().is_steamless_installed() {
						"Run 4GB Patcher"
					} else {
						"Install Steamless by atom0s for 4GB Patcher"
					}
				})
			}
		}
	)
}

fn set_game_location_from_files(mut config: SyncSignal<Config>, e: Event<FormData>) {
	if let Some(files) = &e.files() {
		let files = files.files();
		if let Some(file) = files.first() {
			let exe_path = PathBuf::from(file);
			config.with_mut(move |c| match c.set_path_from_exe(&exe_path) {
				Ok(path) => tracing::info!("Set game location to {}", path.display()),
				Err(e) => tracing::error!("Failed to set game location: {:?}", e),
			});
		}
	}
}

#[component]
pub fn SetGameLocationInput(config: SyncSignal<Config>, id: String) -> Element {
	rsx!(
		input {
			id,
			r#type: "file",
			accept: ".exe",
			multiple: "false",
			hidden: true,
			onchange: move |e| { set_game_location_from_files(config, e) },
			"Set Game Location"
		}
	)
}

#[component]
pub fn SetGameLocationButton(
	class: Option<String>,
	style: Option<String>,
	config: SyncSignal<Config>,
) -> Element {
	// this hack is necessary to use the hidden input pattern
	let id = "hidden-input-id";
	rsx!(
		SetGameLocationInput { config, id: id.to_string() }
		Button {
			class,
			style,
			onclick: move |_| {
				eval(&format!("document.getElementById('{}').click();", id));
			},
			"Set Game Location"
		}
	)
}
