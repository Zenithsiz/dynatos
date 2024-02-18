//! Router example

// Features
#![feature(try_blocks, lazy_cell, lint_reasons)]

// Imports
use {
	dynatos::NodeDynChild,
	dynatos_context::Handle,
	dynatos_html::{html, NodeWithChildren, NodeWithText},
	dynatos_reactive::SignalGet,
	dynatos_router::Location,
	dynatos_util::ObjectSetProp,
	std::cell::LazyCell,
	wasm_bindgen::prelude::wasm_bindgen,
	web_sys::Element,
};

fn main() {
	console_error_panic_hook::set_once();
	dynatos_logger::init();

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
	let location_handle = dynatos_context::provide(location);

	(&body).with_child(
		html::div()
			.with_children([html::p().with_text("Header"), html::hr()])
			.with_dyn_child(self::render_route)
			.with_children([
				html::hr(),
				dynatos_router::anchor("/test").with_text("Test"),
				html::br(),
				dynatos_router::anchor("/cached").with_text("Cached"),
				html::br(),
				dynatos_router::anchor("/empty").with_text("Empty"),
			]),
	);

	#[wasm_bindgen]
	#[expect(dead_code, reason = "We just want to keep the field alive, not use it")]
	struct LocationHandle(Handle<Location>);
	body.set_prop("__dynatos_location_handle", LocationHandle(location_handle));

	Ok(())
}

thread_local! {
	static ROUTE_CACHED: LazyCell<Element> = LazyCell::new(|| self::page("Cached"));
}

fn render_route() -> Option<Element> {
	let location = dynatos_context::with_expect::<Location, _, _>(|location| location.get());

	tracing::info!(%location, "Rendering route");
	match location.path().trim_end_matches('/') {
		// Always re-create page a
		"/test" => Some(self::page("Test")),
		// Cache the 2nd route to show that `dyn_child` can handle the same element fine.
		"/cached" => Some(ROUTE_CACHED.with(|route| LazyCell::force(route).clone())),
		// Have a page without any content
		"/empty" => None,
		// And finally a catch-all page
		page => Some(self::page(&format!("Unknown Page ({page:?})"))),
	}
}

fn page(name: &str) -> Element {
	tracing::info!(%name, "Rendering page");
	html::p().with_text(format!("Page {name}"))
}
