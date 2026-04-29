//! History

// Imports
use {
	crate::{JsValue, Location, Object, PopStateEvent, WebError, object::ObjectFields},
	zutil_inheritance::{FromFields, Value},
	std::sync::{Arc, nonpoison::Mutex},
};

#[derive(Clone, derive_more::Debug)]
#[debug("{_0:p}")]
struct LocationCb(Arc<dyn Fn(PopStateEvent) + Send + Sync>);

#[derive(Clone, Debug)]
struct LocationCbs(Vec<LocationCb>);

zutil_inheritance::value! {
	pub struct History(Object): Send + Sync + Debug {
		location: Location,

		// TODO: Remove this and just move it to the state's event handlers?
		cbs: Mutex<LocationCbs>,
	}
	impl Self {}
}

impl History {
	#[must_use]
	pub fn new(location: Location) -> Self {
		Self::from_fields((
			HistoryFields {
				location,
				cbs: Mutex::new(LocationCbs(vec![])),
			},
			ObjectFields::default(),
		))
	}

	pub fn push_state_with_url(&self, _state: &JsValue, _unused: &str, url: Option<&str>) -> Result<(), WebError> {
		let Some(url) = url else {
			return Ok(());
		};

		// Update the location, and if it  was different, update any listeners
		if self.fields().location.assign_if_different(url.to_owned()) {
			self.update_listeners();
		}

		Ok(())
	}

	/// Updates all listeners
	pub fn update_listeners(&self) {
		let cbs = self.fields().cbs.lock().clone();
		let event = PopStateEvent::default();
		for cb in cbs.0 {
			(cb.0)(event.clone());
		}
	}

	/// Adds a callback for when the location changes
	pub(crate) fn listen(&self, cb: impl Fn(PopStateEvent) + Send + Sync + 'static) {
		self.fields().cbs.lock().0.push(LocationCb(Arc::new(cb)));
	}
}
