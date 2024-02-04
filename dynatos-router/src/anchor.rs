//! Anchor element

// Imports
use {
	crate::Location,
	anyhow::Context,
	dynatos_util::{ev, ElementEventListener, ElementWithAttr, JsResultContext},
	web_sys::{Element, PointerEvent},
};

/// Creates a reactive anchor element.
///
/// Expects a context of type [`Location`](crate::Location).
pub fn anchor<U>(new_location: U) -> Result<Element, anyhow::Error>
where
	U: AsRef<str> + 'static,
{
	let window = web_sys::window().context("Unable to get window")?;
	let document = window.document().context("Unable to get document")?;

	let el = document
		.create_element("a")
		.context("Unable to create a")?
		.with_attr("href", new_location.as_ref())
		.context("Unable to set attribute")?
		.with_event_listener::<ev::Click, _>(move |ev: PointerEvent| {
			ev.prevent_default();
			dynatos_context::with_expect::<Location, _, _>(|location| {
				let new_location = new_location.as_ref();
				if let Err(err) = location.set(new_location) {
					tracing::error!(?new_location, ?err, "Unable to set location");
				}
			});
		});

	Ok(el)
}
