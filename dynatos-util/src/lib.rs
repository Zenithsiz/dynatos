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
use {
	js_sys::Reflect,
	std::{
		fmt,
		hash::{self, Hasher},
	},
	wasm_bindgen::{JsCast, JsValue},
};

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
#[extend::ext(name = ObjectSetProp)]
pub impl js_sys::Object {
	/// Sets the `prop` property of this object to `value`.
	fn set_prop<T>(&self, prop: &str, value: T)
	where
		T: Into<JsValue>,
	{
		let value = value.into();
		Reflect::set(self, &prop.into(), &value)
			.unwrap_or_else(|err| panic!("Unable to set object property {prop:?} to {value:?}: {err:?}"));
	}

	/// Sets the `prop` property of this object to `value`.
	///
	/// Returns the object, for chaining
	fn with_prop<T>(self, prop: &str, value: T) -> Self
	where
		T: Into<JsValue>,
	{
		self.set_prop(prop, value);
		self
	}
}

/// Extension trait to remove a property on an object
#[extend::ext(name = ObjectRemoveProp)]
pub impl js_sys::Object {
	/// Removes the `property` from this object.
	///
	/// Returns if the property existed
	fn remove_prop(&self, property: &str) -> bool {
		Reflect::delete_property(self, &property.into()).expect("Unable to remove object property")
	}
}

/// Error for [`ObjectGet::get`]
#[derive(Clone, Debug)]
pub enum GetError {
	/// Property was missing
	Missing,

	/// Property was the wrong type
	WrongType(JsValue),
}

/// Extension trait to get a property of an object
#[extend::ext(name = ObjectGet)]
pub impl js_sys::Object {
	// TODO: Differentiate between missing value and wrong type?
	fn get<T>(&self, property: &str) -> Result<T, GetError>
	where
		T: JsCast,
	{
		// Note: This returning `Err` should only happen if `self` isn't an object,
		//       which we guarantee, so no errors can occur.
		let value = Reflect::get(self, &property.into()).expect("Unable to get object property");
		if value.is_undefined() {
			return Err(GetError::Missing);
		}

		value.dyn_into().map_err(GetError::WrongType)
	}
}

/// Calculates the hash of a value using the default hasher
pub fn hash_of<T: hash::Hash>(t: &T) -> u64 {
	let mut s = hash::DefaultHasher::new();
	t.hash(&mut s);
	s.finish()
}
