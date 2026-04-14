//! Location

// Imports
use {
	crate::{Object, WebError, object::ObjectFields},
	dynatos_inheritance::{FromFields, Value},
	std::sync::nonpoison::Mutex,
};

dynatos_inheritance::value! {
	pub struct Location(Object): Send + Sync + Debug {
		href: Mutex<String>,
	}
	impl Self {}
}

impl Location {
	#[must_use]
	pub fn new(href: String) -> Self {
		Self::from_fields((LocationFields { href: Mutex::new(href) }, ObjectFields::default()))
	}

	pub fn href(&self) -> Result<String, WebError> {
		Ok(self.fields().href.lock().clone())
	}

	/// Sets the location, if not equal to the current one.
	///
	/// Returns if the new url was different
	#[must_use]
	pub fn assign_if_different(&self, url: String) -> bool {
		let mut href = self.fields().href.lock();
		if *href == url {
			return false;
		}

		*href = url;
		true
	}

	/// Sets the location
	pub fn assign(&self, url: String) -> Result<(), WebError> {
		*self.fields().href.lock() = url;
		Ok(())
	}
}
