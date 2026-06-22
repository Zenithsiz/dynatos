//! `dynatos` web `web-sys` ssr replacement

dynatos_util::web::cfg_ssr! {
	ssr = {
		pub use dynatos_web_ssr::{
			AnimationEvent,
			ClipboardEvent,
			Comment,
			CssStyleDeclaration,
			CssStyleProperties,
			Document,
			DragEvent,
			Element,
			Event,
			EventTarget,
			FocusEvent,
			History,
			HtmlBodyElement,
			HtmlCanvasElement,
			HtmlDetailsElement,
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
			ToggleEvent,
			TransitionEvent,
			WheelEvent,
			Window,
			WebError,
		};
	},
	csr = {
		pub use web_sys::*;
		pub type WebError = wasm_bindgen::JsValue;
	},
}
