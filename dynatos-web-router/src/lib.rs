//! Routing for `dynatos`

#![feature(
	proc_macro_hygiene,
	stmt_expr_attributes,
	negative_impls,
	type_alias_impl_trait,
	trait_alias
)]

mod anchor;
pub mod location;
pub mod query_signal;

pub use self::{
	anchor::anchor,
	location::LocationSignal,
	query_signal::{MultiQuery, QuerySignal, SingleQuery},
};

use js_sys as _;
