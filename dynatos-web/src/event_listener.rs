//! Event listener

// Imports
use {
	crate::{
		DynatosWebCtx,
		types::{ErasableGenericJsValue, EventTarget, FromWasmAbi, WeakRef, cfg_ssr_expr},
	},
	dynatos_sync_types::SyncBounds,
	dynatos_util::TryOrReturnExt,
};

/// Extension trait to define an event listener on an event target with a closure
#[extend::ext(name = EventTargetAddListener)]
pub impl EventTarget {
	/// Adds an event listener to this target
	fn add_event_listener<E>(&self, ctx: &DynatosWebCtx, f: impl SyncBounds + Fn(E::Event) + 'static)
	where
		E: EventListener,
	{
		self.add_event_listener_untyped(ctx, E::name(), f);
	}

	/// Adds an untyped event listener to this target
	fn add_event_listener_untyped<Ev: FromWasmAbi>(
		&self,
		ctx: &DynatosWebCtx,
		event_type: &str,
		f: impl SyncBounds + Fn(Ev) + 'static,
	) {
		cfg_ssr_expr!(
			ssr = self
				.add_event_listener_with_callback(ctx.ssr_state(), event_type, f)
				.expect("Unable to add event listener"),
			csr = {
				use wasm_bindgen::JsCast;

				let _: &DynatosWebCtx = ctx;

				// Build the closure
				let closure = wasm_bindgen::closure::Closure::<dyn Fn(Ev)>::new(f)
					.into_js_value()
					.dyn_into::<js_sys::Function>()
					.expect("Should be a valid function");

				// Then add it
				// TODO: Can this fail? On MDN, nothing seems to mention it can throw.
				self.add_event_listener_with_callback(event_type, &closure)
					.expect("Unable to add event listener");
			}
		);
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
	fn with_event_listener<E>(self, ctx: &DynatosWebCtx, f: impl SyncBounds + Fn(E::Event) + 'static) -> Self
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
	ET: SyncBounds + ErasableGenericJsValue + AsRef<EventTarget> + 'static,
{
	/// Adds an event listener to this target
	fn add_event_listener_el<E, F>(&self, ctx: &DynatosWebCtx, f: F)
	where
		E: EventListener,
		F: SyncBounds + Fn(ET, E::Event) + 'static,
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
	fn with_event_listener_el<E, F>(self, ctx: &DynatosWebCtx, f: F) -> Self
	where
		E: EventListener,
		F: SyncBounds + Fn(ET, E::Event) + 'static,
	{
		self.add_event_listener_el::<E, _>(ctx, f);
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
		crate::types::{
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
