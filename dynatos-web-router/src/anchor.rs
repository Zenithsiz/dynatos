//! Anchor element

// Imports
use {
	crate::Location,
	dynatos_reactive::{SignalBorrow, SignalSet},
	dynatos_web::{ElementWithAttr, EventTargetWithListener, ev, html},
};

/// Creates a reactive anchor element.
///
/// Expects a context of type [`Location`](crate::Location).
pub fn anchor<U>(location: Location, new_location: U) -> web_sys::HtmlElement
where
	U: AsRef<str> + 'static,
{
	html::a()
		.with_attr("href", new_location.as_ref())
		.with_event_listener::<ev!(click)>(move |ev| {
			ev.prevent_default();
			let new_location = new_location.as_ref();
			let res = location.borrow_no_dep().join(new_location);
			match res {
				Ok(new_location) => location.set(new_location),
				Err(err) => tracing::warn!("Unable to join new location to current {new_location:?}: {err}"),
			}
		})
}
