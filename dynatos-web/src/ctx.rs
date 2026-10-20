//! Context

use {
	crate::JsResultContext,
	app_error::{AppError, Context, app_error},
	dynatos_store::ValueStore,
	std::rc::Rc,
	wasm_bindgen::JsCast,
	web_sys::{Document, History, HtmlBodyElement, HtmlHeadElement, Location, Window},
};


#[derive(Debug)]
struct Shared {
	store: ValueStore,
}

#[derive(Debug)]
struct Inner {
	shared:   Shared,
	window:   web_sys::Window,
	document: web_sys::Document,
	head:     web_sys::HtmlHeadElement,
	body:     web_sys::HtmlBodyElement,
	history:  web_sys::History,
	location: web_sys::Location,
}

/// Dynatos web context
///
/// This type serves as a context for interacting with
/// all `dynatos-web-*` crates.
///
/// You should create it once and pass it down to
/// wherever this library requires it.
#[derive(Clone, Debug)]
pub struct DynatosWebCtx(Rc<Inner>);

impl DynatosWebCtx {
	/// Creates a new context type
	pub fn new() -> Result<Self, AppError> {
		let shared = Shared {
			store: ValueStore::new(),
		};

		let window = web_sys::window().context("Missing window")?;
		let document = window.document().context("Missing document")?;
		let head = document.head().context("Missing head")?;
		let head = head
			.dyn_into()
			.map_err(|head| app_error!("Head was not an `HtmlHeadElement`: {head:?}"))?;
		let body = document.body().context("Missing body")?;
		let body = body
			.dyn_into()
			.map_err(|body| app_error!("Body was not an `HtmlBodyElement`: {body:?}"))?;

		let history = window.history().context("Unable to get history")?;
		let location = document.location().context("Missing location")?;

		let inner = Inner {
			shared,
			window,
			document,
			head,
			body,
			history,
			location,
		};

		Ok(Self(Rc::new(inner)))
	}

	/// Returns the context's value store
	#[must_use]
	pub fn store(&self) -> &ValueStore {
		&self.0.shared.store
	}

	/// Returns the window
	#[must_use]
	pub fn window(&self) -> &Window {
		&self.0.window
	}

	/// Returns the document
	#[must_use]
	pub fn document(&self) -> &Document {
		&self.0.document
	}

	/// Returns the head
	#[must_use]
	pub fn head(&self) -> &HtmlHeadElement {
		&self.0.head
	}

	/// Returns the body
	#[must_use]
	pub fn body(&self) -> &HtmlBodyElement {
		&self.0.body
	}

	/// Returns the browser history
	#[must_use]
	pub fn history(&self) -> &History {
		&self.0.history
	}

	/// Returns the browser location
	#[must_use]
	pub fn location(&self) -> &Location {
		&self.0.location
	}
}
