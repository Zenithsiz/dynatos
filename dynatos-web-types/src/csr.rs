//! Client-side rendering types

// Exports
pub use {
	js_sys::{self, Object, WeakRef},
	wasm_bindgen::{self, JsCast, JsValue, convert::FromWasmAbi},
	web_sys::{
		self,
		AnimationEvent,
		ClipboardEvent,
		Comment,
		CssStyleDeclaration,
		Document,
		DragEvent,
		Element,
		Event,
		EventTarget,
		FocusEvent,
		History,
		HtmlBodyElement,
		HtmlCanvasElement,
		HtmlDialogElement,
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
		TransitionEvent,
		WheelEvent,
		Window,
	},
};

pub type WebError = JsValue;

pub trait ErasableGenericJsValue = wasm_bindgen::ErasableGeneric<Repr = wasm_bindgen::JsValue>;
