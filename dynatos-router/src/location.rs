//! Location

// Imports
use {
	anyhow::Context,
	dynatos_reactive::{Signal, SignalUpdate, SignalWith},
	dynatos_util::JsResultContext,
	url::Url,
	wasm_bindgen::JsValue,
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
		let window = web_sys::window().context("Unable to get window")?;
		let document = window.document().context("Unable to get document")?;
		let location = Self::parse_location_url(document)?;

		let inner = Inner { location };
		let inner = Signal::new(inner);

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
		let document = window.document().context("Unable to get document")?;
		let history = window.history().context("Unable to get history")?;

		// Push the new location into history
		history
			.push_state_with_url(&JsValue::UNDEFINED, "", Some(new_location.as_ref()))
			.expect("Unable to push history");

		// Then parse the location back
		let new_location = Self::parse_location_url(document)?;
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
	fn parse_location_url(document: web_sys::Document) -> Result<Url, anyhow::Error> {
		let location = document.location().context("Document had no location")?;
		let location = location.href().context("Unable to get location href")?;
		let location = location.parse::<Url>().context("Location href was an invalid url")?;
		Ok(location)
	}
}
