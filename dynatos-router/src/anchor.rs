//! Anchor element

// Imports
use {
	crate::Location,
	dynatos_html::{ev, html, ElementWithAttr, EventTargetWithListener},
	dynatos_reactive::{SignalBorrow, SignalSet},
	web_sys::Element,
};

/// Creates a reactive anchor element.
///
/// Expects a context of type [`Location`](crate::Location).
pub fn anchor<U>(new_location: U) -> Element
where
	U: AsRef<str> + 'static,
{
	html::a()
		.with_attr("href", new_location.as_ref())
		.with_event_listener::<ev::Click>(move |ev| {
			ev.prevent_default();
			dynatos_context::with_expect::<Location, _, _>(|location| {
				let new_location = new_location.as_ref();
				let res = location.borrow().join(new_location);
				match res {
					Ok(new_location) => location.set(new_location),
					Err(err) => tracing::warn!("Unable to join new location to current {new_location:?}: {err}"),
				}
			});
		})
}
