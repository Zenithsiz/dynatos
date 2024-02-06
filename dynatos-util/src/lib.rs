//! Utilities for [`dynatos`]

// Features
#![feature(decl_macro, never_type, try_trait_v2, control_flow_enum)]

// Modules
mod event_listener;
pub mod try_or_return;
pub mod weak_ref;

// Exports
pub use self::{
	event_listener::{ev, EventListener, EventTargetAddListener},
	try_or_return::{TryOrReturn, TryOrReturnExt},
	weak_ref::WeakRef,
};

// Imports
use {js_sys::Reflect, std::fmt, wasm_bindgen::JsValue};

/// Extension trait to be able to use `.context` on `Result<T, JsValue>`.
#[extend::ext(name = JsResultContext)]
pub impl<T> Result<T, JsValue> {
	fn context<C>(self, context: C) -> Result<T, anyhow::Error>
	where
		C: fmt::Display + Send + Sync + 'static,
	{
		self.map_err(|err| {
			let err = format!("{err:?}");
			let err = anyhow::Error::msg(err);
			err.context(context)
		})
	}
}

/// Extension trait to set a property on an object
#[extend::ext(name = ObjectSet)]
pub impl js_sys::Object {
	fn set<T>(&self, property: &str, value: T)
	where
		T: Into<JsValue>,
	{
		Reflect::set(self, &property.into(), &value.into()).expect("Unable to set object property");
	}
}
