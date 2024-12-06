//! Counter example

// Features
#![feature(stmt_expr_attributes, proc_macro_hygiene)]

// Imports
use {
	dynatos_html::{html, NodeWithChildren, NodeWithText},
	dynatos_reactive::{Signal, SignalBorrowMut, SignalGet, SignalSet},
	dynatos_util::{cloned, ev, EventTargetWithListener, JsResultContext},
	tracing_subscriber::prelude::*,
	web_sys::Element,
};

fn main() {
	console_error_panic_hook::set_once();
	tracing_subscriber::registry()
		.with(
			tracing_subscriber::fmt::layer()
				.with_ansi(false)
				.without_time()
				.with_level(false)
				.with_writer(tracing_web::MakeWebConsoleWriter::new().with_pretty_level()),
		)
		.init();

	match self::run() {
		Ok(()) => tracing::info!("Successfully initialized"),
		Err(err) => tracing::error!("Unable to start: {err:?}"),
	}
}

fn run() -> Result<(), anyhow::Error> {
	let window = web_sys::window().expect("Unable to get window");
	let document = window.document().expect("Unable to get document");
	let body = document.body().expect("Unable to get document body");

	let counter = self::counter();
	body.append_child(&counter).context("Unable to append counter")?;

	Ok(())
}

fn counter() -> Element {
	let value = Signal::new(0);
	html::div().with_children([
		#[cloned(value)]
		html::button()
			.with_text("Clear")
			.with_event_listener::<ev::Click>(move |_ev| value.set(0)),
		#[cloned(value)]
		html::button()
			.with_text("+")
			.with_event_listener::<ev::Click>(move |_ev| *value.borrow_mut() += 1),
		#[cloned(value)]
		html::button()
			.with_text("-")
			.with_event_listener::<ev::Click>(move |_ev| *value.borrow_mut() -= 1),
		html!("<span>Value: %{value.get()}%.</span>"),
	])
}
