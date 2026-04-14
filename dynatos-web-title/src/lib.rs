//! Title management for `dynatos`

// TODO: It seems that titles aren't getting dropped for some reason.

// Features
#![feature(macro_attr)]
#![cfg_attr(not(feature = "sync"), feature(thread_local))]

// Imports
use {
	dynatos_sync_types::{IMut, thread_local_or_global},
	dynatos_util::HoleyStack,
	dynatos_web::{
		DynatosWebCtx,
		ObjectSetProp,
		types::{Object, cfg_ssr_expr},
	},
};

/// Title stack.
#[thread_local_or_global]
static TITLE_STACK: IMut<HoleyStack<String>> = IMut::new(HoleyStack::new());

/// Title.
///
/// Sets the title for as long as this lives.
#[derive(Debug)]
pub struct Title {
	/// Title index
	title_idx: usize,
	ctx:       DynatosWebCtx,
}

impl Title {
	/// Creates a title.
	pub fn new<S>(ctx: &DynatosWebCtx, title: S) -> Self
	where
		S: Into<String>,
	{
		let title: String = title.into();

		// If no title exists, add the current one
		let mut stack = TITLE_STACK.lock();
		if stack.is_empty() {
			stack.push(ctx.document().title());
		}

		// Then set the title and add ours to the stack
		ctx.document().set_title(&title);
		let title_idx = stack.push(title);

		Self {
			title_idx,
			ctx: ctx.clone(),
		}
	}
}

impl Drop for Title {
	fn drop(&mut self) {
		// Remove our title
		let mut stack = TITLE_STACK.lock();
		let _prev_title = stack.pop(self.title_idx).expect("Title was already taken");

		// Then find the next title to set back to.
		let next_title = stack.top().expect("Should contain at least 1 title");
		self.ctx.document().set_title(next_title);
	}
}

/// Extension trait to attach a title to an object.
#[extend::ext(name = ObjectAttachTitle)]
pub impl Object {
	/// Attaches a title to this object
	fn attach_title(&self, ctx: &DynatosWebCtx, title: &str) {
		let prop_name = "__dynatos_web_title";
		let title = Title::new(ctx, title);

		let title = cfg_ssr_expr!(
			ssr = {
				use dynatos_web::types::JsValue;
				JsValue::from_any(title)
			},
			csr = {
				/// A wasm `Title` type.
				#[wasm_bindgen::prelude::wasm_bindgen]
				#[expect(dead_code, reason = "We just want to keep the field alive, not use it")]
				struct WasmTitle(Title);

				WasmTitle(title)
			}
		);

		self.set_prop(prop_name, title);
	}
}

/// Extension trait to attach a title to an object.
#[extend::ext(name = ObjectWithTitle)]
pub impl<T> T
where
	T: AsRef<Object>,
{
	/// Attaches a title to this object.
	///
	/// Returns the object, for chaining
	fn with_title(self, ctx: &DynatosWebCtx, title: &str) -> Self {
		self.as_ref().attach_title(ctx, title);
		self
	}
}
