//! Event listener

use {
	crate::{DynatosWebCtx, util},
	dynatos_util::TryOrReturnExt,
	js_sys::WeakRef,
	wasm_bindgen::{ErasableGeneric, JsValue, convert::FromWasmAbi},
	web_sys::EventTarget,
};

/// Extension trait to define an event listener on an event target with a closure
#[extend::ext(name = EventTargetAddListener)]
pub impl EventTarget {
	/// Adds an event listener to this target
	fn add_event_listener<E>(&self, ctx: &DynatosWebCtx, f: impl Fn(E::Event) + 'static)
	where
		E: EventListener,
	{
		self.add_event_listener_untyped(ctx, E::name(), f);
	}

	/// Adds an untyped event listener to this target
	fn add_event_listener_untyped<Ev: FromWasmAbi>(
		&self,
		_ctx: &DynatosWebCtx,
		event_type: &str,
		f: impl Fn(Ev) + 'static,
	) {
		// TODO: Can this fail? On MDN, nothing seems to mention it can throw.
		self.add_event_listener_with_callback(event_type, &util::js_fn::<dyn Fn(Ev)>(f))
			.expect("Unable to add event listener");
	}
}

/// Extension trait to define an event listener on an event target with a closure
#[extend::ext(name = EventTargetWithListener)]
pub impl<T> T
where
	T: AsRef<EventTarget>,
{
	/// Adds an event listener to this target
	///
	/// Returns the type, for chaining
	fn with_event_listener<E>(self, ctx: &DynatosWebCtx, f: impl Fn(E::Event) + 'static) -> Self
	where
		E: EventListener,
	{
		self.as_ref().add_event_listener::<E>(ctx, f);
		self
	}
}

/// Extension trait to define an event listener on an element with a closure
#[extend::ext(name = ElementAddListener)]
pub impl<ET> ET
where
	ET: ErasableGeneric<Repr = JsValue> + AsRef<EventTarget> + 'static,
{
	/// Adds an event listener to this target
	fn add_event_listener_el<E>(&self, ctx: &DynatosWebCtx, f: impl Fn(ET, E::Event) + 'static)
	where
		E: EventListener,
	{
		// Build the closure
		// Note: Important that `el` is a weak reference here, else we
		//       create a circular reference from node <-> event listener.
		let el = WeakRef::new(self);
		<ET as AsRef<EventTarget>>::as_ref(self).add_event_listener::<E>(ctx, move |ev| {
			let el = el.deref().or_return()?;
			f(el, ev);
		});
	}

	/// Adds an event listener to this target
	///
	/// Returns the type, for chaining
	fn with_event_listener_el<E>(self, ctx: &DynatosWebCtx, f: impl Fn(ET, E::Event) + 'static) -> Self
	where
		E: EventListener,
	{
		self.add_event_listener_el::<E>(ctx, f);
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
	use super::EventListener;

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
		load: web_sys::Event;
		click: web_sys::PointerEvent;
		change: web_sys::Event;
		input: web_sys::InputEvent;
		submit: web_sys::SubmitEvent;
		blur: web_sys::FocusEvent;
		dblclick: web_sys::MouseEvent;
		wheel: web_sys::WheelEvent;
		paste: web_sys::ClipboardEvent;
		drop: web_sys::DragEvent;
		dragstart: web_sys::DragEvent;
		dragover: web_sys::DragEvent;
		popstate: web_sys::PopStateEvent;
		pointermove: web_sys::PointerEvent;
		pointerdown: web_sys::PointerEvent;
		pointerup: web_sys::PointerEvent;
		pointerout: web_sys::PointerEvent;
		animationend: web_sys::AnimationEvent;
		transitionend: web_sys::TransitionEvent;
		toggle: web_sys::ToggleEvent;
		contextmenu: web_sys::PointerEvent;
	}
}
