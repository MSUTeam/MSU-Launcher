use std::collections::VecDeque;

use dioxus::prelude::*;

pub struct InfoLog {
	infos: VecDeque<String>,
	errors: VecDeque<String>,
	max_len: usize,
}

impl InfoLog {
	pub fn new(max_len: usize) -> Self {
		Self {
			infos: VecDeque::new(),
			errors: VecDeque::new(),
			max_len,
		}
	}

	pub fn info<T>(&mut self, string: T)
	where
		T: Into<String>,
	{
		if self.infos.len() >= self.max_len {
			self.infos.pop_back();
		}
		self.infos.push_front(string.into());
	}

	pub fn error<T>(&mut self, string: T)
	where
		T: Into<String>,
	{
		if self.errors.len() >= self.max_len {
			self.errors.pop_back();
		}
		self.errors.push_front(string.into());
	}

	pub fn latest_info(&self) -> &str {
		self.infos.front().map(|s| s.as_str()).unwrap_or_default()
	}

	pub fn latest_error(&self) -> &str {
		self.errors.front().map(|s| s.as_str()).unwrap_or_default()
	}
}

#[component]
pub fn InfoPanel(
	class: Option<String>,
	style: Option<String>,
	logger: SyncSignal<InfoLog>,
) -> Element {
	let class = class.unwrap_or_default();
	rsx! {
		div { class: "{class} info-panel", style,
			div { {logger.read().latest_info()} }
			div { {logger.read().latest_error()} }
		}
	}
}
