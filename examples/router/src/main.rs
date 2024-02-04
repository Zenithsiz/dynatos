//! Router example

// Features
#![feature(try_blocks)]

// Imports
use {
	anyhow::Context,
	dynatos::ElementDynChild,
	dynatos_context::Handle,
	dynatos_html::{html, ElementWithChildren, ElementWithTextContent},
	dynatos_reactive::SignalGet,
	dynatos_router::Location,
	dynatos_util::{JsResultContext, ObjectDefineProperty},
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
	let window = web_sys::window().context("Unable to get window")?;
	let document = window.document().context("Unable to get document")?;
	let body = document.body().context("Unable to get document body")?;

	let location = Location::new().context("Unable to create location")?;
	let location_handle = dynatos_context::provide(location);

	body.dyn_child(move || match self::render_route() {
		Ok(page) => page,
		Err(err) => html::p().with_text_content(format!("Error: {err:?}")),
	});

	#[wasm_bindgen]
	struct LocationHandle(Handle<Location>);
	body.define_property("__dynatos_location_handle", LocationHandle(location_handle));

	Ok(())
}

fn render_route() -> Result<Element, anyhow::Error> {
	let location = dynatos_context::with_expect::<Location, _, _>(|location| location.get());

	let page = match location.path().trim_end_matches('/') {
		"/a" => self::page("A").context("Unable to build button")?,
		"/b" => self::page("B").context("Unable to build button")?,
		page => self::page(&format!("Unknown Page ({page:?})")).context("Unable to build button")?,
	};

	Ok(page)
}

fn page(name: &str) -> Result<Element, anyhow::Error> {
	html::div()
		.with_children([
			html::p().with_text_content(format!("Page {name}")),
			html::hr(),
			dynatos_router::anchor("/a")?.with_text_content("A"),
			html::br(),
			dynatos_router::anchor("/b")?.with_text_content("B"),
		])
		.context("Unable to add children")
}
