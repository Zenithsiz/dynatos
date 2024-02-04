//! Counter example

// Imports
use {
	anyhow::Context,
	dynatos::ElementDynText,
	dynatos_html::html,
	dynatos_reactive::Signal,
	dynatos_util::{ev, ElementEventListener, ElementWithChildren, ElementWithTextContent, JsResultContext},
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

	let counter = self::counter().context("Unable to build button")?;
	body.append_child(&counter).context("Unable to append counter")?;

	Ok(())
}

fn counter() -> Result<Element, anyhow::Error> {
	let value = Signal::new(0);
	html::div()
		.with_children([
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
		.context("Unable to add children")
}
