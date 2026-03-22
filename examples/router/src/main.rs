//! Router example

// Features
#![feature(try_blocks, thread_local)]

// Imports
use {
	app_error::AppError,
	core::cell::LazyCell,
	dynatos_html::{NodeWithChildren, NodeWithText, html},
	dynatos_html_reactive::{NodeWithDynChildren, ObjectWithContext},
	dynatos_html_router::Location,
	dynatos_reactive::SignalGetCloned,
	tracing_subscriber::prelude::*,
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

	body.with_context(location).with_child(
		html::div()
			.with_children([html::p().with_text("Header"), html::hr()])
			.with_dyn_children(self::render_route)
			.with_children([
				html::hr(),
				dynatos_html_router::anchor("/test").with_text("Test"),
				html::br(),
				dynatos_html_router::anchor("/cached").with_text("Cached"),
				html::br(),
				dynatos_html_router::anchor("/empty").with_text("Empty"),
			]),
	);

	Ok(())
}

#[thread_local]
static ROUTE_CACHED: LazyCell<web_sys::HtmlElement> = LazyCell::new(|| self::page("Cached"));


fn render_route() -> Option<web_sys::HtmlElement> {
	let location = dynatos_context::with_expect::<Location, _, _>(|location| location.get_cloned());

	tracing::info!(%location, "Rendering route");
	match location.path().trim_end_matches('/') {
		// Always re-create page a
		"/test" => Some(self::page("Test")),
		// Cache the 2nd route to show that `dyn_child` can handle the same element fine.
		"/cached" => Some(LazyCell::force(&ROUTE_CACHED).clone()),
		// Have a page without any content
		"/empty" => None,
		// And finally a catch-all page
		page => Some(self::page(&format!("Unknown Page ({page:?})"))),
	}
}

fn page(name: &str) -> web_sys::HtmlElement {
	tracing::info!(%name, "Rendering page");
	html::p().with_text(format!("Page {name}"))
}
