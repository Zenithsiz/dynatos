//! Query example

// Features
#![feature(try_blocks, stmt_expr_attributes, proc_macro_hygiene)]

// Imports
use {
	dynatos::{NodeWithDynText, ObjectWithContext},
	dynatos_html::{ev, html, EventTargetWithListener, NodeWithChildren, NodeWithText},
	dynatos_reactive::{SignalBorrowMut, SignalGet, SignalSet, SignalWithDefault},
	dynatos_router::{Location, QuerySignal},
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

	let location = Location::new();

	body.with_context::<Location>(location).with_child(self::page());

	Ok(())
}

fn page() -> Element {
	let query = QuerySignal::<i32>::new("a").with_default(20);

	html::div().with_children([
		#[cloned(query)]
		html::p().with_dyn_text(move || format!("{:?}", query.get())),
		html::hr(),
		dynatos_router::anchor("/?a=5").with_text("5"),
		html::br(),
		dynatos_router::anchor("/?a=7").with_text("7"),
		html::br(),
		dynatos_router::anchor("/?a=abc").with_text("abc"),
		html::br(),
		#[cloned(query)]
		html::button()
			.with_event_listener::<ev::Click>(move |_ev| {
				*query.borrow_mut() += 1;
			})
			.with_text("Add"),
		html::br(),
		html::button()
			.with_event_listener::<ev::Click>(move |_ev| {
				query.set(6);
			})
			.with_text("6"),
	])
}
