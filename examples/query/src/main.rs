//! Query example

// Features
#![feature(stmt_expr_attributes, proc_macro_hygiene)]

// Imports
use {
	app_error::AppError,
	dynatos_loadable::Loadable,
	dynatos_reactive::{SignalBorrowMut, SignalGetCloned, SignalSet},
	dynatos_web::{EventTargetWithListener, NodeWithChildren, NodeWithText, ev, html},
	dynatos_web_reactive::NodeWithDynText,
	dynatos_web_router::{Location, QuerySignal, SingleQuery},
	tracing_subscriber::prelude::*,
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

	let location = Location::new();

	body.with_child(self::page(location));

	Ok(())
}

fn page(location: Location) -> web_sys::HtmlElement {
	// TODO: If we add `.with_loadable_default()`, use it again in this example.
	let query = SingleQuery::<i32>::new(location.clone(), "a");
	let query = QuerySignal::new(location.clone(), query);

	html::div().with_children([
		#[cloned(query)]
		html::p().with_dyn_text(move || format!("{:?}", query.get_cloned())),
		html::hr(),
		dynatos_web_router::anchor(location.clone(), "/?a=5").with_text("5"),
		html::br(),
		dynatos_web_router::anchor(location.clone(), "/?a=7").with_text("7"),
		html::br(),
		dynatos_web_router::anchor(location, "/?a=abc").with_text("abc"),
		html::br(),
		#[cloned(query)]
		html::button()
			.with_event_listener::<ev!(click)>(move |_ev| {
				if let Loadable::Loaded(value) = &mut *query.borrow_mut() {
					*value += 1;
				}
			})
			.with_text("Add"),
		html::br(),
		html::button()
			.with_event_listener::<ev!(click)>(move |_ev| {
				query.set(6);
			})
			.with_text("6"),
	])
}
