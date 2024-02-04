//! Router example

// Features
#![feature(try_blocks)]

// Imports
use {
	dynatos::ElementDynChild,
	dynatos_context::Handle,
	dynatos_html::{html, ElementWithChildren, ElementWithTextContent},
	dynatos_reactive::SignalGet,
	dynatos_router::Location,
	dynatos_util::ObjectDefineProperty,
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

	body.dyn_child(self::render_route);

	#[wasm_bindgen]
	struct LocationHandle(Handle<Location>);
	body.define_property("__dynatos_location_handle", LocationHandle(location_handle));

	Ok(())
}

fn render_route() -> Element {
	let location = dynatos_context::with_expect::<Location, _, _>(|location| location.get());

	match location.path().trim_end_matches('/') {
		"/a" => self::page("A"),
		"/b" => self::page("B"),
		page => self::page(&format!("Unknown Page ({page:?})")),
	}
}

fn page(name: &str) -> Element {
	html::div().with_children([
		html::p().with_text_content(format!("Page {name}")),
		html::hr(),
		dynatos_router::anchor("/a").with_text_content("A"),
		html::br(),
		dynatos_router::anchor("/b").with_text_content("B"),
	])
}
