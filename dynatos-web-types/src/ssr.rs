//! Server side rendering types

// Exports
pub use dynatos_web_ssr::{
	AnimationEvent,
	ClipboardEvent,
	Comment,
	Document,
	DragEvent,
	Element,
	Event,
	EventTarget,
	FocusEvent,
	History,
	HtmlBodyElement,
	HtmlCanvasElement,
	HtmlElement,
	HtmlHeadElement,
	HtmlImageElement,
	HtmlInputElement,
	HtmlTextAreaElement,
	InputEvent,
	JsValue,
	Location,
	MouseEvent,
	Node,
	Object,
	PointerEvent,
	PopStateEvent,
	SubmitEvent,
	Text,
	WeakRef,
	WebError,
	WheelEvent,
	Window,
};

// Imports
use dynatos_inheritance::Value;

pub trait JsCast = Value;
pub trait FromWasmAbi = Value;
pub trait ErasableGenericJsValue = AsRef<Object> + Value + Clone;
