//! `dynatos` web `wasm-bindgen` ssr replacement

#![feature(trait_alias)]


dynatos_util::web::cfg_ssr! {
	ssr = {
				use zutil_inheritance::Value;

				pub use dynatos_web_ssr::JsValue;


		pub trait JsCast = Value;
		pub trait ErasableGeneric: Value {
			type Repr;
		}

		impl<T: Value> ErasableGeneric for T {
			type Repr = T;
		}

		pub mod convert {
						use zutil_inheritance::Value;

			pub trait FromWasmAbi = Value;
		}
	},
	csr = {
		pub use wasm_bindgen::*;
	},
}
