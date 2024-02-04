//! Location

// Imports
use {
	anyhow::Context,
	dynatos_reactive::{Signal, SignalGet, SignalSet, SignalUpdate, SignalWith},
	dynatos_util::{ev, EventTargetAddListener},
	url::Url,
	wasm_bindgen::JsValue,
	web_sys::PopStateEvent,
};

/// Inner
struct Inner {
	/// Location
	location: Url,
}

/// Location
#[derive(Clone)]
pub struct Location(Signal<Inner>);

impl Location {
	/// Creates a new location
	pub fn new() -> Result<Self, anyhow::Error> {
		let location = self::parse_location_url();
		let inner = Inner { location };
		let inner = Signal::new(inner);

		// Add an event listener on the document for when the user navigates manually
		let window = web_sys::window().context("Unable to get window")?;
		window.add_event_listener::<ev::PopState, _>({
			let inner = inner.clone();
			move |_ev: PopStateEvent| {
				let new_location = self::parse_location_url();
				inner.update(|inner| inner.location = new_location);
			}
		});

		Ok(Self(inner))
	}
}

impl SignalGet for Location {
	type Value = Url;

	fn get(&self) -> Self::Value {
		self.with(|location| location.clone())
	}
}

impl SignalWith for Location {
	type Value = Url;

	fn with<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&Self::Value) -> O,
	{
		self.0.with(|inner| f(&inner.location))
	}
}

impl SignalSet for Location {
	// TODO: This feels like a weird part of the api, maybe we should use a custom type like `RedirectUrl` or something?
	type Value = String;

	fn set(&self, new_location: Self::Value) {
		let window = web_sys::window().expect("Unable to get window");
		let history = window.history().expect("Unable to get history");

		// Push the new location into history
		history
			.push_state_with_url(&JsValue::UNDEFINED, "", Some(new_location.as_ref()))
			.expect("Unable to push history");

		// Then parse the location back
		let new_location = self::parse_location_url();
		self.0.update(|inner| inner.location = new_location);
	}
}

impl SignalUpdate for Location {
	type Value = Url;

	fn update<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&mut Self::Value) -> O,
	{
		self.0.update(|inner| {
			let output = f(&mut inner.location);

			let window = web_sys::window().expect("Unable to get window");
			let history = window.history().expect("Unable to get history");

			// Push the new location into history
			history
				.push_state_with_url(&JsValue::UNDEFINED, "", Some(inner.location.as_ref()))
				.expect("Unable to push history");

			output
		})
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
