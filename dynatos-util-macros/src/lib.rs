//! Macros for `dynatos-util`

// Features
#![feature(if_let_guard, try_blocks)]

// Modules
mod cloned;

// Imports
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn cloned(attr: TokenStream, input: TokenStream) -> TokenStream {
	cloned::cloned(attr, input)
}
