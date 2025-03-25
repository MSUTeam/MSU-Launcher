use dioxus::prelude::*;
use std::fmt::Write;
use tokio::sync::broadcast;
use tracing::level_filters::LevelFilter;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
	layer::{Context, SubscriberExt},
	Layer,
};

static LOG_CHANNEL: once_cell::sync::Lazy<(
	broadcast::Sender<LogUpdate>,
	broadcast::Receiver<LogUpdate>,
)> = once_cell::sync::Lazy::new(|| broadcast::channel(100));

struct MessageVisitor<'a>(&'a mut String);

impl tracing::field::Visit for MessageVisitor<'_> {
	fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
		if field.name() == "message" {
			let _ = write!(self.0, "{:?}", value);
		}
	}
}

struct FilteringLayer<Inner> {
	inner: Inner,
}

impl<Inner> FilteringLayer<Inner> {
	fn new(inner: Inner) -> Self {
		Self { inner }
	}
}
// workaround for https://github.com/DioxusLabs/dioxus/issues/2566
impl<S, Inner> Layer<S> for FilteringLayer<Inner>
where
	S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
	Inner: Layer<S>,
{
	fn on_event(&self, event: &tracing::Event<'_>, ctx: Context<'_, S>) {
		let should_filter = event.metadata().fields().iter().any(|field| {
			if field.name() == "message" {
				let mut message = String::new();
				let mut visitor = MessageVisitor(&mut message);
				event.record(&mut visitor);
				message.contains("Error parsing user_event: Error(\"invalid type: null, expected usize\", line: 0, column: 0)")
			} else {
				false
			}
		});
		if !should_filter {
			self.inner.on_event(event, ctx);
		}
	}
}

pub(crate) static TRACING: once_cell::sync::Lazy<()> = once_cell::sync::Lazy::new(|| {
	let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "msu_launcher.log");
	let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
	let file_layer = FilteringLayer::new(
		tracing_subscriber::fmt::layer()
			.with_writer(non_blocking)
			.with_ansi(false),
	);

	let env_filter = tracing_subscriber::EnvFilter::builder()
		.with_default_directive(LevelFilter::INFO.into())
		.parse("")
		.unwrap();

	let console_layer = FilteringLayer::new(tracing_subscriber::fmt::layer());
	let info_logger = FilteringLayer::new(InfoLog::new(LOG_CHANNEL.0.clone()));

	let subscriber = tracing_subscriber::Registry::default()
		.with(env_filter)
		.with(console_layer)
		.with(file_layer)
		.with(info_logger);

	tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

	GUARD.with(|g| {
		*g.borrow_mut() = Some(guard);
	});
});

thread_local! {
	static GUARD: std::cell::RefCell<Option<tracing_appender::non_blocking::WorkerGuard>> = const { std::cell::RefCell::new(None) };
}

#[derive(Clone)]
enum LogUpdate {
	Info(Box<str>),
	Error(Box<str>),
}

struct InfoLog {
	sender: broadcast::Sender<LogUpdate>,
}

impl InfoLog {
	pub fn new(sender: broadcast::Sender<LogUpdate>) -> Self {
		Self { sender }
	}
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for InfoLog {
	fn on_event(
		&self,
		event: &tracing::Event<'_>,
		_ctx: tracing_subscriber::layer::Context<'_, S>,
	) {
		let mut message = String::new();
		let mut visitor = MessageVisitor(&mut message);
		event.record(&mut visitor);
		let message = message.into_boxed_str();

		let update = match *event.metadata().level() {
			tracing::Level::ERROR => LogUpdate::Error(message),
			tracing::Level::INFO => LogUpdate::Info(message),
			_ => {
				return;
			}
		};
		let _ = self.sender.send(update);
	}
}

#[component]
pub fn InfoPanel(class: Option<String>, style: Option<String>) -> Element {
	let class = class.unwrap_or_default();
	let mut last_error = use_signal(|| "".into());
	let mut last_info = use_signal(|| "".into());

	use_future(move || async move {
		let mut rx = LOG_CHANNEL.1.resubscribe();
		while let Ok(udpate) = rx.recv().await {
			match udpate {
				LogUpdate::Info(info) => {
					last_info.set(info);
				}
				LogUpdate::Error(error) => {
					last_error.set(error);
				}
			}
		}
	});
	rsx! {
		div { class: "{class} info-panel", style,
			div { {last_info.read()} }
			div { {last_error.read()} }
		}
	}
}
