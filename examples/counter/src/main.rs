//! Counter example

// Features
#![feature(stmt_expr_attributes, proc_macro_hygiene)]

// Imports
use {
	dynatos_html::{html, JsResultContext},
	dynatos_reactive::{Signal, SignalBorrowMut, SignalGet, SignalSet},
	tracing_subscriber::prelude::*,
	web_sys::Element,
	zutil_cloned::cloned,
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
	let value = Signal::<_>::new(0);

	#[cloned(value)]
	let clear = move |_ev| value.set(0);
	#[cloned(value)]
	let add = move |_ev| *value.borrow_mut() += 1;
	#[cloned(value)]
	let sub = move |_ev| *value.borrow_mut() -= 1;

	html!(
		"<div>
			<button @Click=clear>Clear</button>
			<button @Click=add>+</button>
			<button @Click=sub>-</button>
			<span>Value: %{value.get()}%.</span>
		</div>"
	)
}
