//! Anchor element

// Imports
use {
	crate::Location,
	dynatos_html::{html, ElementWithAttr},
	dynatos_reactive::SignalSet,
	dynatos_util::{ev, EventTargetAddListener},
	web_sys::{Element, PointerEvent},
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
		.with_event_listener::<ev::Click, _>(move |ev: PointerEvent| {
			ev.prevent_default();
			dynatos_context::with_expect::<Location, _, _>(|location| {
				let new_location = new_location.as_ref();
				location.set(new_location);
			});
		})
}
