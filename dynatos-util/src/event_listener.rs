//! Event listener

// Imports
use wasm_bindgen::{
	closure::{Closure, IntoWasmClosure, WasmClosure},
	JsCast,
};

/// Extension trait to define an event listener on an event target with a closure
#[extend::ext(name = EventTargetAddListener)]
pub impl web_sys::EventTarget {
	/// Adds an event listener to this target
	fn add_event_listener<E, F>(&self, f: F)
	where
		E: EventListener,
		F: IntoWasmClosure<E::Closure> + 'static,
	{
		// Build the closure
		let closure = Closure::<E::Closure>::new(f)
			.into_js_value()
			.dyn_into::<web_sys::js_sys::Function>()
			.expect("Should be a valid function");

		// Then add it
		// TODO: Can this fail? On MDN, nothing seems to mention it can throw.
		self.add_event_listener_with_callback(E::name(), &closure)
			.expect("Unable to add event listener");
	}
}
/// Extension trait to define an event listener on an event target with a closure
#[extend::ext(name = EventTargetWithListener)]
pub impl<T> T
where
	T: AsRef<web_sys::EventTarget>,
{
	/// Adds an event listener to this target
	///
	/// Returns the type, for chaining
	fn with_event_listener<E>(self, f: impl IntoWasmClosure<E::Closure> + 'static) -> Self
	where
		E: EventListener,
	{
		self.as_ref().add_event_listener::<E, _>(f);
		self
	}
}

/// Event listener
pub trait EventListener {
	/// Closure type
	type Closure: ?Sized + WasmClosure;

	/// Returns the event name
	fn name() -> &'static str;
}

/// Events
pub mod ev {
	// Imports
	use {
		super::EventListener,
		web_sys::{InputEvent, PointerEvent, PopStateEvent},
	};

	macro define_events(
		$(
			$( #[ doc = $ev_doc:literal ] )*
			$Event:ident($ArgTy:ty) = $name:literal;
		)*
	) {
		$(
			$( #[ doc = $ev_doc ] )*
			pub struct $Event;

			impl EventListener for $Event {
				type Closure = dyn Fn($ArgTy);

				fn name() -> &'static str {
					$name
				}
			}
		)*
	}

	define_events! {
		/// `click` Event
		Click(PointerEvent) = "click";

		/// `input` Event
		Input(InputEvent) = "input";

		/// `popstate` Event
		PopState(PopStateEvent) = "popstate";
	}
}
