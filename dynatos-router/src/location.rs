//! Location

// Imports
use {
	anyhow::Context,
	dynatos_reactive::{Signal, SignalUpdate, SignalWith},
	dynatos_util::{ev, EventTargetAddListener, JsResultContext},
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
		let location = Self::parse_location_url();
		let inner = Inner { location };
		let inner = Signal::new(inner);

		// Add an event listener on the document for when the user navigates manually
		let window = web_sys::window().context("Unable to get window")?;
		window.add_event_listener::<ev::PopState, _>({
			let inner = inner.clone();
			move |_ev: PopStateEvent| {
				let new_location = Self::parse_location_url();
				inner.update(|inner| inner.location = new_location);
			}
		});

		Ok(Self(inner))
	}

	/// Gets the location as a url
	pub fn get(&self) -> Url {
		self.with(|location| location.clone())
	}

	/// Uses the location as a url
	pub fn with<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&Url) -> O,
	{
		self.0.with(|inner| f(&inner.location))
	}

	/// Sets the location.
	///
	/// `new_location` may be relative.
	pub fn set<U>(&self, new_location: U) -> Result<(), anyhow::Error>
	where
		U: AsRef<str>,
	{
		let window = web_sys::window().context("Unable to get window")?;
		let history = window.history().context("Unable to get history")?;

		// Push the new location into history
		history
			.push_state_with_url(&JsValue::UNDEFINED, "", Some(new_location.as_ref()))
			.expect("Unable to push history");

		// Then parse the location back
		let new_location = Self::parse_location_url();
		self.0.update(|inner| {
			inner.location = new_location;
		});

		Ok(())
	}

	/// Updates the location
	pub fn update<F, O>(&self, f: F) -> Result<O, anyhow::Error>
	where
		F: FnOnce(&mut Url) -> O,
	{
		self.0.update(|inner| {
			let output = f(&mut inner.location);

			let window = web_sys::window().context("Unable to get window")?;
			let history = window.history().context("Unable to get history")?;

			// Push the new location into history
			history
				.push_state_with_url(&JsValue::UNDEFINED, "", Some(inner.location.as_ref()))
				.expect("Unable to push history");

			Ok(output)
		})
	}

	/// Parses the location as url
	fn parse_location_url() -> Url {
		let window = web_sys::window().expect("Unable to get window");
		let document = window.document().expect("Unable to get document");

		let location = document.location().expect("Document had no location");
		let location = location.href().expect("Unable to get location href");
		location.parse::<Url>().expect("Location href was an invalid url")
	}
}
