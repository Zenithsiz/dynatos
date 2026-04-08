//! Context

// Imports
use {
	app_error::{AppError, Context, app_error},
	dynatos_sync_types::RcPtr,
	wasm_bindgen::JsCast,
};

/// Inner representation
///
/// This is separate from the actual type so
/// we can put it inside of an `Rc`, to reduce
/// the size of [`DynatosWeb`]
#[derive(Debug)]
struct Inner {
	window:   web_sys::Window,
	document: web_sys::Document,
	body:     web_sys::HtmlBodyElement,
}

/// Dynatos web context
///
/// This type serves as a context for interacting with
/// all `dynatos-web-*` crates.
///
/// You should create it once and pass it down to
/// wherever this library requires it.
#[derive(Clone, Debug)]
pub struct DynatosWebCtx(RcPtr<Inner>);

impl DynatosWebCtx {
	/// Creates a new context type
	pub fn new() -> Result<Self, AppError> {
		let window = web_sys::window().context("Missing window")?;
		let document = window.document().context("Missing document")?;
		let body = document.body().context("Missing body")?;
		let body = body
			.dyn_into()
			.map_err(|body| app_error!("Body was not an `HtmlBodyElement`: {body:?}"))?;

		let inner = Inner { window, document, body };
		Ok(Self(RcPtr::new(inner)))
	}

	/// Returns the window
	#[must_use]
	pub fn window(&self) -> &web_sys::Window {
		&self.0.window
	}

	/// Returns the document
	#[must_use]
	pub fn document(&self) -> &web_sys::Document {
		&self.0.document
	}

	/// Returns the body
	#[must_use]
	pub fn body(&self) -> &web_sys::HtmlBodyElement {
		&self.0.body
	}
}
