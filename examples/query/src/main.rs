//! Query example

// Features
#![feature(try_blocks)]

// Imports
use {
	dynatos::NodeDynText,
	dynatos_context::Handle,
	dynatos_html::{html, ElementWithChildren, ElementWithTextContent},
	dynatos_reactive::{SignalGet, SignalSet, SignalUpdate, SignalWithDefault},
	dynatos_router::{Location, QuerySignal},
	dynatos_util::{ev, EventTargetAddListener, ObjectDefineProperty},
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

	let child = self::page();
	body.append_child(&child).expect("Unable to append child");

	#[wasm_bindgen]
	struct LocationHandle(Handle<Location>);
	body.define_property("__dynatos_location_handle", LocationHandle(location_handle));

	Ok(())
}

fn page() -> Element {
	let query = QuerySignal::<i32>::new("a").with_default(20);

	html::div().with_children([
		html::p().with_dyn_text({
			let query = query.clone();
			move || Some(format!("{:?}", query.get()))
		}),
		html::hr(),
		dynatos_router::anchor("/?a=5").with_text_content("5"),
		html::br(),
		dynatos_router::anchor("/?a=7").with_text_content("7"),
		html::br(),
		dynatos_router::anchor("/?a=abc").with_text_content("abc"),
		html::br(),
		html::button()
			.with_event_listener::<ev::Click, _>({
				let query = query.clone();
				move |_ev| {
					query.update(|value| *value += 1);
				}
			})
			.with_text_content("Add"),
		html::br(),
		html::button()
			.with_event_listener::<ev::Click, _>(move |_ev| {
				query.set(6);
			})
			.with_text_content("6"),
	])
}
