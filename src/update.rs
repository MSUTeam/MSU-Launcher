use anyhow::{Context, Result};
use dioxus::prelude::*;

use crate::button::Button;

const API_URL: &str = "https://api.github.com/repos/MSUTeam/MSU-Launcher/releases/latest";
const RELEASE_URL: &str = "https://www.nexusmods.com/battlebrothers/mods/729?tab=files";

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

pub async fn check_update_available() -> Result<bool> {
	let client = reqwest::Client::builder()
		.user_agent(APP_USER_AGENT)
		.build()
		.context("Couldn't build reqwest agent for update check")?;
	let response = client
		.get(API_URL)
		.send()
		.await
		.context("Failed to send update request")?;
	let json: serde_json::Value = response
		.json()
		.await
		.context("Failed to parse update response")?;
	let latest_version = json["tag_name"]
		.as_str()
		.context("tag_name missing from GitHub API response")?;
	let latest_version = semver::Version::parse(latest_version).with_context(|| {
		format!(
			"Couldn't parse latest version ({}) as semver ",
			latest_version
		)
	})?;
	Ok(latest_version
		> semver::Version::parse(env!("CARGO_PKG_VERSION"))
			.context("Couldn't parse current version")?)
}

#[component]
pub fn UpdateButton(
	#[props(default = "".to_string())] class: String,
	style: Option<String>,
) -> Element {
	let mut disabled = use_signal(|| true);
	use_future(move || async move {
		match check_update_available().await {
			Ok(update_available) => disabled.set(!update_available),
			Err(e) => tracing::error!("Failed to check for updates: {:?}", e),
		}
	});
	rsx!(
		div { class: "{class} flex flex-row items-center", style,
			if !*disabled.read() {
				img { class: "w-10 h-10", src: "assets/gfx/icons/warning.svg" }
			}
			Button {
				onclick: |_| {
					match open::that(RELEASE_URL) {
						Ok(_) => tracing::info!("Opened NexusMods"),
						Err(e) => tracing::error!("Failed to open NexusMods: {}", e),
					}
				},
				disabled,
				{
					if *disabled.read() {
						"No Update Available"
					} else {
						"Update Available, Download Here!"
					}
				}
			}
		}
	)
}
