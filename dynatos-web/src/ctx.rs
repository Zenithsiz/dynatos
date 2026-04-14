//! Context

// Imports
use {
	crate::types::{Document, History, HtmlBodyElement, HtmlHeadElement, Location, Window, cfg_ssr, cfg_ssr_expr},
	app_error::AppError,
	dynatos_sync_types::RcPtr,
};

cfg_ssr! {
	ssr = {
		#[derive(Clone, Debug)]
		struct Inner {
			state: dynatos_web_ssr::State,
		}
	},
	csr = {
		#[derive(Clone, Debug)]
		struct Inner {
			window:   web_sys::Window,
			document: web_sys::Document,
			head:     web_sys::HtmlHeadElement,
			body:     web_sys::HtmlBodyElement,
			history:  web_sys::History,
			location: web_sys::Location,
		}
	}
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
	pub fn new(#[cfg(feature = "ssr")] ssr_state: dynatos_web_ssr::State) -> Result<Self, AppError> {
		let inner = cfg_ssr_expr!(
			ssr = Inner { state: ssr_state },
			csr = {
				use {
					crate::JsResultContext,
					app_error::{Context, app_error},
					wasm_bindgen::JsCast,
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

				Inner {
					window,
					document,
					head,
					body,
					history,
					location,
				}
			}
		);

		Ok(Self(RcPtr::new(inner)))
	}

	/// Returns the window
	#[must_use]
	pub fn window(&self) -> &Window {
		cfg_ssr_expr!(ssr = self.0.state.window(), csr = &self.0.window)
	}

	/// Returns the document
	#[must_use]
	pub fn document(&self) -> &Document {
		cfg_ssr_expr!(ssr = self.0.state.document(), csr = &self.0.document)
	}

	/// Returns the head
	#[must_use]
	pub fn head(&self) -> &HtmlHeadElement {
		cfg_ssr_expr!(ssr = self.0.state.head(), csr = &self.0.head)
	}

	/// Returns the body
	#[must_use]
	pub fn body(&self) -> &HtmlBodyElement {
		cfg_ssr_expr!(ssr = self.0.state.body(), csr = &self.0.body)
	}

	/// Returns the browser history
	#[must_use]
	pub fn history(&self) -> &History {
		cfg_ssr_expr!(ssr = self.0.state.history(), csr = &self.0.history)
	}

	/// Returns the browser location
	#[must_use]
	pub fn location(&self) -> &Location {
		cfg_ssr_expr!(ssr = self.0.state.location(), csr = &self.0.location)
	}

	#[cfg(feature = "ssr")]
	#[must_use]
	pub fn ssr_state(&self) -> &dynatos_web_ssr::State {
		&self.0.state
	}
}
