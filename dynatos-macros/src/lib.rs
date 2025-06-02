//! Macros for [`dynatos`]

// Imports
use {derive_utils::quick_derive, proc_macro::TokenStream};

#[proc_macro_derive(ToDynNode)]
pub fn derive_iterator(input: TokenStream) -> TokenStream {
	quick_derive! {
		input,
		dynatos::ToDynNode,
		trait ToDynNode {
			fn to_node(&self) -> Option<web_sys::Node>;
		}
	}
}
