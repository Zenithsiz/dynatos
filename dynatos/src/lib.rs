//! Dynatos framework

// Features
#![feature(let_chains, unboxed_closures, associated_type_bounds)]

// Modules
pub mod dyn_element;
mod element_dyn_attr;
mod node_dyn_child;
mod node_dyn_text;
mod object_attach_effect;
mod object_dyn_prop;

// Exports
pub use self::{
	dyn_element::dyn_element,
	element_dyn_attr::ElementDynAttr,
	node_dyn_child::{AsDynNode, AsOptNode, NodeDynChild},
	node_dyn_text::{NodeDynText, WithDynText},
	object_attach_effect::ObjectAttachEffect,
	object_dyn_prop::ObjectDynProp,
};
