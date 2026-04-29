//! `dynatos` web SSR

// Features
#![feature(
	decl_macro,
	const_trait_impl,
	more_qualified_paths,
	trivial_bounds,
	nonpoison_mutex,
	sync_nonpoison,
	macro_derive
)]

// Modules
pub mod css_style;
pub mod document;
pub mod element;
pub mod event;
pub mod event_target;
pub mod history;
pub mod html_element;
pub mod js_value;
pub mod location;
pub mod node;
pub mod object;
pub mod state;
pub mod weak_ref;
pub mod window;

// Exports
pub use self::{
	css_style::{CssStyleDeclaration, CssStyleProperties},
	document::Document,
	element::Element,
	event::{
		AnimationEvent,
		ClipboardEvent,
		DragEvent,
		Event,
		FocusEvent,
		InputEvent,
		MouseEvent,
		PointerEvent,
		PopStateEvent,
		SubmitEvent,
		TransitionEvent,
		WheelEvent,
	},
	event_target::EventTarget,
	history::History,
	html_element::{
		HtmlBodyElement,
		HtmlCanvasElement,
		HtmlDialogElement,
		HtmlElement,
		HtmlHeadElement,
		HtmlImageElement,
		HtmlInputElement,
		HtmlTextAreaElement,
	},
	js_value::JsValue,
	location::Location,
	node::Node,
	object::Object,
	state::State,
	weak_ref::WeakRef,
	window::Window,
};

// Imports
#[cfg(feature = "reactive")]
use std::collections::HashMap;
use {
	self::{event_target::EventTargetFields, node::NodeFields, object::ObjectFields},
	app_error::AppError,
	core::any::Any,
	std::sync::nonpoison::Mutex,
	zutil_inheritance::FromFields,
};

zutil_inheritance::value! {
	pub struct Text(Node, EventTarget, Object): Send + Sync + Debug {
		contents: Option<String>,
	}
	impl Self {}
}

impl Text {
	#[must_use]
	pub fn new(contents: Option<String>) -> Self {
		Self::from_fields((
			TextFields { contents },
			NodeFields::new("#text"),
			EventTargetFields::default(),
			ObjectFields::default(),
		))
	}
}

zutil_inheritance::value! {
	pub struct Comment(Node, EventTarget, Object): Send + Sync + Debug {
		contents: Option<String>,
	}
	impl Self {}
}

impl Comment {
	#[must_use]
	pub fn new(contents: Option<String>) -> Self {
		Self::from_fields((
			CommentFields { contents },
			NodeFields::new("#comment"),
			EventTargetFields::default(),
			ObjectFields::default(),
		))
	}
}


#[derive(Clone)]
#[derive(derive_more::Debug, derive_more::From)]
#[debug("{_0:?}")]
pub struct WebError(pub AppError);

// TODO: These types need to be defined inside this crate due to a compiler bug
//       that results in a conflicting implementation due to a wrong projection
//       of the parent storage/vtable.
zutil_inheritance::value! {
	pub struct ObjectAttachValueValues(Object): Send + Sync + Debug {
		values: Mutex<Vec<Box<dyn Any + Send + Sync>>>,
	}
	impl Self {}
}

#[cfg(feature = "reactive")]
zutil_inheritance::value! {
	pub struct ObjectAttachEffectEffects(Object): Send + Sync + Debug {
		effects: Mutex<HashMap<usize, dynatos_reactive::Effect>>,
	}
	impl Self {}
}
