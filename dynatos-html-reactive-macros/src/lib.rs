//! Macros for [`dynatos_html_reactive`]

// Imports
use {derive_utils::quick_derive, proc_macro::TokenStream};

#[proc_macro_derive(ToDynNode)]
pub fn derive_iterator(input: TokenStream) -> TokenStream {
	quick_derive! {
		input,
		dynatos_html_reactive::ToDynNode,
		trait ToDynNode {
			fn to_node(&self) -> Option<web_sys::Node>;
		}
	}
}
