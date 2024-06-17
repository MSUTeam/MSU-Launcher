use dioxus::prelude::*;
use std::{path::PathBuf, process::Command};

use crate::{patcher_laa, patcher_preload, Config, InfoLog};

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
	logger: SyncSignal<InfoLog>,
) -> Element {
	let mut counter: i32 = 0;
	rsx!(
		Button {
			class,
			style,
			onclick: move |_| {
			    println!("Config");
			    logger
			        .with_mut(|l| {
			            l.info(format!("Config! {}", counter));
			            counter += 1;
			            l.error(format!("Config! {}", counter));
			        })
			},
			"Config"
		}
	)
}

async fn launch_game(config: ReadOnlySignal<Config, SyncStorage>, mut logger: SyncSignal<InfoLog>) {
	patcher_preload::async_gather_and_create_mod(config, logger).await;
	let exe_path = match config.read().get_bb_exe_path() {
		Some(path) => path,
		None => {
			logger.with_mut(|l| {
				l.error("Couldn't find BattleBrothers.exe");
			});
			return;
		}
	};
	match Command::new(exe_path.as_ref()).spawn() {
		Ok(_) => {
			logger.with_mut(|l| {
				l.info("Launched Battle Brothers");
			});
		}
		Err(e) => {
			logger.with_mut(|l| {
				l.error(format!("Couldn't launch Battle Brothers: {}", e));
			});
		}
	}
}

#[component]
pub fn LaunchButton(
	class: Option<String>,
	style: Option<String>,
	config: ReadOnlySignal<Config, SyncStorage>,
	logger: SyncSignal<InfoLog>,
) -> Element {
	rsx!(
		Button {
			class,
			style,
			disabled: use_memo(move || !config.read().bb_path_known()),
			onclick: move |_| {
			    spawn(async move {
			        let _ = tokio::spawn(async move {
			                launch_game(config, logger).await;
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
	logger: SyncSignal<InfoLog>,
) -> Element {
	rsx!(
		Button {
			class,
			style,
			disabled: use_memo(move || !config.read().bb_path_known()),
			onclick: move |_| {
			    spawn(async move {
			        patcher_preload::mt_gather_and_create_mod(config, logger).await
			    });
			},
			"Run Preload Patcher"
		}
	)
}

#[component]
pub fn Run4GBPatcherButton(
	class: Option<String>,
	style: Option<String>,
	config: ReadOnlySignal<Config, SyncStorage>,
	logger: SyncSignal<InfoLog>,
) -> Element {
	rsx!(
		Button {
			class,
			style,
			disabled: use_memo(move || !config.read().bb_path_known()),
			onclick: move |_| {
			    spawn(async move {
			        let _ = patcher_laa::patch_from_config(config, logger);
			    });
			},
			"Run 4GB Patcher"
		}
	)
}

fn set_game_location_from_files(
	mut config: SyncSignal<Config>,
	mut logger: SyncSignal<InfoLog>,
	e: Event<FormData>,
) {
	if let Some(files) = &e.files() {
		let files = files.files();
		if let Some(file) = files.first() {
			let exe_path = PathBuf::from(file);
			config.with_mut(move |c| {
				logger.with_mut(move |l| {
					match c.set_path_from_exe(&exe_path) {
						Ok(path) => l.info(format!("Set game location to {}", path.display())),
						Err(e) => l.error(format!("Failed to set game location: {:?}", e)),
					};
				})
			});
		}
	}
}

#[component]
pub fn SetGameLocationInput(
	config: SyncSignal<Config>,
	logger: SyncSignal<InfoLog>,
	id: String,
) -> Element {
	rsx!(
		input {
			id,
			r#type: "file",
			accept: ".exe",
			multiple: "false",
			hidden: true,
			onchange: move |e| { set_game_location_from_files(config, logger, e) },
			"Set Game Location"
		}
	)
}

#[component]
pub fn SetGameLocationButton(
	class: Option<String>,
	style: Option<String>,
	config: SyncSignal<Config>,
	logger: SyncSignal<InfoLog>,
) -> Element {
	// this hack is necessary to use the hidden input pattern
	let id = "hidden-input-id";
	rsx!(
		SetGameLocationInput { config, logger, id: id.to_string() }
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
