//! Event listener

// Imports
use {
	crate::WeakRef,
	dynatos_util::TryOrReturnExt,
	wasm_bindgen::{JsCast, closure::Closure, convert::FromWasmAbi},
	web_sys::js_sys,
};

/// Extension trait to define an event listener on an event target with a closure
#[extend::ext(name = EventTargetAddListener)]
pub impl<T> T
where
	T: AsRef<web_sys::EventTarget>,
{
	/// Adds an event listener to this target
	fn add_event_listener<E>(&self, f: impl Fn(E::Event) + 'static)
	where
		E: EventListener,
	{
		// Build the closure
		let closure = Closure::<dyn Fn(E::Event)>::new(f)
			.into_js_value()
			.dyn_into::<js_sys::Function>()
			.expect("Should be a valid function");

		// Then add it
		// TODO: Can this fail? On MDN, nothing seems to mention it can throw.
		self.as_ref()
			.add_event_listener_with_callback(E::name(), &closure)
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
	fn with_event_listener<E>(self, f: impl Fn(E::Event) + 'static) -> Self
	where
		E: EventListener,
	{
		self.add_event_listener::<E>(f);
		self
	}
}

/// Extension trait to define an event listener on an element with a closure
#[extend::ext(name = ElementAddListener)]
pub impl<ET> ET
where
	ET: AsRef<web_sys::EventTarget> + AsRef<js_sys::Object> + JsCast + 'static,
{
	/// Adds an event listener to this target
	fn add_event_listener_el<E, F>(&self, f: F)
	where
		E: EventListener,
		F: Fn(ET, E::Event) + 'static,
	{
		// Build the closure
		// Note: Important that `el` is a weak reference here, else we
		//       create a circular reference from node <-> event listener.
		let el = WeakRef::new(self);
		<ET as AsRef<web_sys::EventTarget>>::as_ref(self).add_event_listener::<E>(move |ev| {
			let el = el.get().or_return()?;
			f(el, ev);
		});
	}

	/// Adds an event listener to this target
	///
	/// Returns the type, for chaining
	fn with_event_listener_el<E>(self, f: impl Fn(ET, E::Event) + 'static) -> Self
	where
		E: EventListener,
	{
		self.add_event_listener_el::<E, _>(f);
		self
	}
}

/// Event listener
pub trait EventListener {
	/// Event type
	type Event: FromWasmAbi + 'static;

	/// Returns the event name
	fn name() -> &'static str;
}

pub macro ev($Event:ident) {
	self::ev::$Event
}

/// Events
#[expect(
	nonstandard_style,
	reason = "Macro-generated code, the types aren't accessed directly and instead go through a macro"
)]
mod ev {
	// Imports
	use {
		super::EventListener,
		web_sys::{
			ClipboardEvent,
			DragEvent,
			Event,
			FocusEvent,
			InputEvent,
			MouseEvent,
			PointerEvent,
			PopStateEvent,
			SubmitEvent,
			WheelEvent,
		},
	};

	macro define_events(
		$(
			$Event:ident: $ArgTy:ty;
		)*
	) {
		$(
			pub struct $Event;

			impl EventListener for $Event {
				type Event = $ArgTy;

				fn name() -> &'static str {
					stringify!($Event)
				}
			}
		)*
	}

	define_events! {
		load: Event;
		click: PointerEvent;
		change: Event;
		input: InputEvent;
		submit: SubmitEvent;
		blur: FocusEvent;
		dblclick: MouseEvent;
		wheel: WheelEvent;
		paste: ClipboardEvent;
		drop: DragEvent;
		dragstart: DragEvent;
		dragover: DragEvent;
		popstate: PopStateEvent;
		pointermove: PointerEvent;
		pointerdown: PointerEvent;
		pointerup: PointerEvent;
		pointerout: PointerEvent;
	}
}
