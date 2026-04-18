//! Client-side rendering types

// Exports
pub use {
	js_sys::{self, Object, WeakRef},
	wasm_bindgen::{self, JsCast, JsValue, closure::Closure, convert::FromWasmAbi},
	web_sys::{
		self,
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
		Location,
		MouseEvent,
		Node,
		PointerEvent,
		PopStateEvent,
		SubmitEvent,
		Text,
		WheelEvent,
		Window,
	},
};

pub type WebError = JsValue;

pub trait ErasableGenericJsValue = wasm_bindgen::ErasableGeneric<Repr = wasm_bindgen::JsValue>;
