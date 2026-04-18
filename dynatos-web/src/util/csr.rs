//! CSR utilities

// Imports
use {
	core::marker::Unsize,
	wasm_bindgen::{
		JsCast,
		closure::{IntoWasmClosure, WasmClosure},
	},
};

/// Converts a rust function into a javascript function
pub fn js_fn<Fn: ?Sized + WasmClosure>(f: impl Unsize<Fn> + IntoWasmClosure<Fn> + 'static) -> js_sys::Function {
	wasm_bindgen::closure::Closure::<Fn>::new(f)
		.into_js_value()
		.dyn_into::<js_sys::Function>()
		.expect("Should be a valid function")
}
