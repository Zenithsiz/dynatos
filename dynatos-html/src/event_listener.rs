//! Event listener

// Imports
use {
	crate::WeakRef,
	dynatos_util::TryOrReturnExt,
	wasm_bindgen::{closure::Closure, convert::FromWasmAbi, JsCast},
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

/// Events
pub mod ev {
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
			$( #[ doc = $ev_doc:literal ] )*
			$Event:ident($ArgTy:ty) = $name:literal;
		)*
	) {
		$(
			$( #[ doc = $ev_doc ] )*
			pub struct $Event;

			impl EventListener for $Event {
				type Event = $ArgTy;

				fn name() -> &'static str {
					$name
				}
			}
		)*
	}

	define_events! {
		/// `load` Event
		Load(Event) = "load";

		/// `click` Event
		Click(PointerEvent) = "click";

		/// `change` Event
		Change(Event) = "change";

		/// `input` Event
		Input(InputEvent) = "input";

		/// `submit` Event
		Submit(SubmitEvent) = "submit";

		/// `blur` Event
		Blur(FocusEvent) = "blur";

		/// `dblclick` Event
		DoubleClick(MouseEvent) = "dblclick";

		/// `wheel` Event
		Wheel(WheelEvent) = "wheel";

		/// `paste` Event
		Paste(ClipboardEvent) = "paste";

		/// `drop` Event
		Drop(DragEvent) = "drop";

		/// `dragstart` Event
		DragStart(DragEvent) = "dragstart";

		/// `dragover` Event
		DragOver(DragEvent) = "dragover";

		/// `popstate` Event
		PopState(PopStateEvent) = "popstate";

		/// `pointermove` event
		PointerMove(PointerEvent) = "pointermove";

		/// `pointerdown` event
		PointerDown(PointerEvent) = "pointerdown";

		/// `pointerup` event
		PointerUp(PointerEvent) = "pointerup";

		/// `pointerout` event
		PointerOut(PointerEvent) = "pointerout";
	}
}
