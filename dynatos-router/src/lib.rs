//! Routing for `dynatos`

// Features
#![feature(
	proc_macro_hygiene,
	stmt_expr_attributes,
	let_chains,
	negative_impls,
	type_alias_impl_trait,
	trait_alias
)]

// Modules
mod anchor;
pub mod location;
pub mod query_signal;

// Exports
pub use self::{
	anchor::anchor,
	location::Location,
	query_signal::{MultiQuery, QuerySignal, SingleQuery},
};
