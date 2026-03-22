//! Macros for [`dynatos_web_reactive`]

// Imports
use {derive_utils::quick_derive, proc_macro::TokenStream};

#[proc_macro_derive(WithDynNodes)]
pub fn derive_iterator(input: TokenStream) -> TokenStream {
	quick_derive! {
		input,
		dynatos_web_reactive::WithDynNodes,
		trait WithDynNodes {
			fn with_children(&self, mut f: impl FnMut(web_sys::Node));
		}
	}
}
