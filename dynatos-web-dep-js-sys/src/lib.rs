//! `dynatos` web `js-sys` ssr replacement

dynatos_util::web::cfg_ssr! {
	ssr = {
		pub use dynatos_web_ssr::{Object, WeakRef};
	},
	csr = {
		pub use js_sys::*;
	},
}
