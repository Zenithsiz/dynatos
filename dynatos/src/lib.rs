//! Dynatos framework

// Features
#![feature(unboxed_closures, unsize, never_type)]

// Modules
mod element_dyn_attr;
mod element_dyn_children;
mod html_element_dyn_css_prop;
mod node_dyn_child;
mod node_dyn_text;
mod object_attach_context;
mod object_attach_effect;
mod object_attach_value;
mod object_dyn_prop;
mod with_dyn_pred;
mod with_dyn_text;

// Exports
pub use {
	self::{
		element_dyn_attr::{ElementDynAttr, ElementWithDynAttr},
		element_dyn_children::{ElementDynChildren, ElementWithDynChildren, WithDynNodes},
		html_element_dyn_css_prop::{HtmlElementDynCssProp, HtmlElementWithDynCssProp},
		node_dyn_child::{NodeDynChild, NodeWithDynChild, ToDynNode},
		node_dyn_text::{NodeDynText, NodeWithDynText},
		object_attach_context::{ObjectAttachContext, ObjectWithContext},
		object_attach_effect::{ObjectAttachEffect, ObjectWithEffect},
		object_attach_value::{ObjectAttachValue, ObjectWithValue},
		object_dyn_prop::{ObjectDynProp, ObjectWithDynProp, ToDynProp},
		with_dyn_pred::WithDynPred,
		with_dyn_text::WithDynText,
	},
	dynatos_macros::*,
};
