//! Counter example

// Imports
use {
	dynatos::NodeDynText,
	dynatos_html::{html, ElementWithChildren, ElementWithTextContent},
	dynatos_reactive::{Signal, SignalGet, SignalSet, SignalUpdate},
	dynatos_util::{ev, EventTargetAddListener, JsResultContext},
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

	let counter = self::counter();
	body.append_child(&counter).context("Unable to append counter")?;

	Ok(())
}

fn counter() -> Element {
	let value = Signal::new(0);
	html::div().with_children([
		html::button()
			.with_text_content("Clear")
			.with_event_listener::<ev::Click, _>({
				let value = value.clone();
				move |_ev| {
					value.set(0);
				}
			}),
		html::button()
			.with_text_content("+")
			.with_event_listener::<ev::Click, _>({
				let value = value.clone();
				move |_ev| value.update(|value| *value += 1)
			}),
		html::button()
			.with_text_content("-")
			.with_event_listener::<ev::Click, _>({
				let value = value.clone();
				move |_ev| value.update(|value| *value -= 1)
			}),
		html::span().with_dyn_text({
			let value = value.clone();
			move || Some(format!("Value: {}.", value.get()))
		}),
	])
}
