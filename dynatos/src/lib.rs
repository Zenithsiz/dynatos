//! Dynatos framework

// Features
#![feature(let_chains, unboxed_closures, associated_type_bounds, lint_reasons)]

// Modules
mod element_dyn_attr;
mod node_dyn_child;
mod node_dyn_text;
mod object_attach_effect;
mod object_dyn_prop;

// Exports
pub use self::{
	element_dyn_attr::ElementDynAttr,
	node_dyn_child::{NodeDynChild, ToDynNode},
	node_dyn_text::{NodeDynText, WithDynText},
	object_attach_effect::ObjectAttachEffect,
	object_dyn_prop::ObjectDynProp,
};
