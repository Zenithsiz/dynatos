//! Dynatos framework

// Features
#![feature(let_chains, unboxed_closures, associated_type_bounds, lint_reasons, unsize)]

// Modules
mod element_dyn_attr;
mod element_dyn_children;
mod node_dyn_child;
mod node_dyn_text;
mod object_attach_context;
mod object_attach_effect;
mod object_dyn_prop;

// Exports
pub use self::{
	element_dyn_attr::{ElementDynAttr, ElementWithDynAttr},
	element_dyn_children::{ElementDynChildren, ElementWithDynChildren, WithDynNodes},
	node_dyn_child::{NodeDynChild, NodeWithDynChild, ToDynNode},
	node_dyn_text::{NodeDynText, NodeWithDynText, WithDynText},
	object_attach_context::{ObjectAttachContext, ObjectWithContext},
	object_attach_effect::{ObjectAttachEffect, ObjectWithEffect},
	object_dyn_prop::{ObjectDynProp, ObjectWithDynProp, ToDynProp},
};
