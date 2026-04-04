//! Macros for [`dynatos_inheritance`]

// Modules
mod value;

#[proc_macro]
pub fn value(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	value::def(input)
}
