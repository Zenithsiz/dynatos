//! Dynamic children example

// Features
#![feature(stmt_expr_attributes, proc_macro_hygiene)]

// Imports
use {
	app_error::AppError,
	dynatos_html::{JsResultContext, NodeWithChildren, NodeWithText, html},
	dynatos_html_reactive::NodeWithDynChildren,
	dynatos_reactive::{Signal, SignalBorrowMut, SignalGet, SignalSet},
	tracing_subscriber::prelude::*,
	web_sys::HtmlElement,
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

fn run() -> Result<(), AppError> {
	let window = web_sys::window().expect("Unable to get window");
	let document = window.document().expect("Unable to get document");
	let body = document.body().expect("Unable to get document body");

	let parent = self::parent();
	body.append_child(&parent).context("Unable to append counter")?;

	Ok(())
}

fn parent() -> HtmlElement {
	let base = Signal::new(0);
	let num_children = Signal::new(2);

	html::div()
		.with_child(self::counter("Number of children", num_children.clone()))
		.with_child(self::counter("Base", base.clone()))
		.with_child(html::hr())
		.with_dyn_children(move || self::child(base.get(), num_children.get()))
		.with_child(html::hr())
		.with_child(html::p().with_text("Footer"))
}

fn child(base: i32, num_children: i32) -> Vec<HtmlElement> {
	(0..num_children)
		.map(|idx| html::p().with_text(format!("{}", base + idx)))
		.collect()
}

fn counter(name: &str, value: Signal<i32>) -> HtmlElement {
	#[cloned(value)]
	let reset = move |_ev| value.set(0);
	#[cloned(value)]
	let add = move |_ev| *value.borrow_mut() += 1;
	#[cloned(value)]
	let sub = move |_ev| *value.borrow_mut() -= 1;

	html!(
		r#"<div>
			<button @click="reset">Reset</button>
			<button @click="add">+</button>
			<button @click="sub">-</button>
			<span>%{static name}%: %{value.get()}%.</span>
		</div>"#
	)
}
