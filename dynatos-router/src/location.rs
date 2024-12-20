//! Location

// Imports
use {
	core::ops::{Deref, DerefMut},
	dynatos_html::{ev, EventTargetAddListener},
	dynatos_reactive::{signal, Signal, SignalBorrow, SignalBorrowMut, SignalUpdate, SignalWith},
	url::Url,
	wasm_bindgen::JsValue,
	zutil_cloned::cloned,
};

/// Inner
#[derive(Debug)]
struct Inner {
	/// Location
	location: Url,
}

/// Location
#[derive(Clone)]
pub struct Location(Signal<Inner>);

impl Location {
	/// Creates a new location
	#[expect(
		clippy::new_without_default,
		reason = "We want locations to only be created explicitly"
	)]
	#[must_use]
	#[track_caller]
	pub fn new() -> Self {
		let location = self::parse_location_url();
		let inner = Inner { location };
		let inner = Signal::new(inner);

		// Add an event listener on the document for when the user navigates manually
		let window = web_sys::window().expect("Unable to get window");
		#[cloned(inner)]
		window.add_event_listener::<ev::PopState>(move |_ev| {
			let new_location = self::parse_location_url();
			inner.borrow_mut().location = new_location;
		});

		Self(inner)
	}
}

/// Reference type for [`SignalBorrow`] impl
#[derive(Debug)]
pub struct BorrowRef<'a>(signal::BorrowRef<'a, Inner>);

impl Deref for BorrowRef<'_> {
	type Target = Url;

	fn deref(&self) -> &Self::Target {
		&self.0.location
	}
}

impl SignalBorrow for Location {
	type Ref<'a>
		= BorrowRef<'a>
	where
		Self: 'a;

	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_> {
		BorrowRef(self.0.borrow())
	}
}

impl SignalWith for Location {
	type Value<'a> = &'a Url;

	#[track_caller]
	fn with<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let location = self.borrow();
		f(&location)
	}
}

/// Reference type for [`SignalBorrowMut`] impl
#[derive(Debug)]
pub struct BorrowRefMut<'a>(signal::BorrowRefMut<'a, Inner>);

impl Deref for BorrowRefMut<'_> {
	type Target = Url;

	fn deref(&self) -> &Self::Target {
		&self.0.location
	}
}

impl DerefMut for BorrowRefMut<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0.location
	}
}

impl Drop for BorrowRefMut<'_> {
	fn drop(&mut self) {
		let window = web_sys::window().expect("Unable to get window");
		let history = window.history().expect("Unable to get history");

		// Push the new location into history
		match history.push_state_with_url(&JsValue::UNDEFINED, "", Some(self.0.location.as_str())) {
			Ok(()) => tracing::info!("Pushed history: {:?}", self.0.location.as_str()),
			Err(err) => tracing::error!("Unable to push history {:?}: {err:?}", self.0.location.as_str()),
		}
	}
}

impl SignalBorrowMut for Location {
	type RefMut<'a>
		= BorrowRefMut<'a>
	where
		Self: 'a;

	#[track_caller]
	fn borrow_mut(&self) -> Self::RefMut<'_> {
		let value = self.0.borrow_mut();
		BorrowRefMut(value)
	}
}

impl SignalUpdate for Location {
	type Value<'a> = &'a mut Url;

	#[track_caller]
	fn update<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let mut location = self.borrow_mut();
		f(&mut location)
	}
}

/// Parses the location as url
fn parse_location_url() -> Url {
	let window = web_sys::window().expect("Unable to get window");
	let document = window.document().expect("Unable to get document");

	let location = document.location().expect("Document had no location");
	let location = location.href().expect("Unable to get location href");
	location.parse::<Url>().expect("Location href was an invalid url")
}
