//! Events

// Imports
use crate::Object;

dynatos_inheritance::value! {
	pub struct Event(Object): Send + Sync + Debug + Default {}
	impl Self {}
}

impl Event {
	pub const fn prevent_default(&self) {
		// Do we need to do anything here?
	}
}

decl_events! {
	new;

	AnimationEvent,
	ClipboardEvent,
	DragEvent,
	FocusEvent,
	InputEvent,
	MouseEvent,
	PointerEvent,
	PopStateEvent,
	SubmitEvent,
	WheelEvent,
}

macro decl_events($new:ident; $($Name:ident),* $(,)?) {
	$(
		dynatos_inheritance::value! {
			pub struct $Name(Event, Object): Send + Sync + Debug + Default {}
			impl Self {}
		}
	)*
}
