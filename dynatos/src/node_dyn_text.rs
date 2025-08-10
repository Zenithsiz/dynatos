//! Node reactive text

// Imports
use {
	crate::{ObjectAttachEffect, WithDynText},
	dynatos_html::WeakRef,
	dynatos_reactive::Effect,
	dynatos_util::TryOrReturnExt,
};

/// Extension trait to add reactive text to a node
#[extend::ext(name = NodeDynText)]
pub impl web_sys::Node {
	/// Adds dynamic text to this node
	#[track_caller]
	fn set_dyn_text<T>(&self, text: T)
	where
		T: WithDynText + 'static,
	{
		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the node.
		//       Otherwise, the node will be keeping us alive, while we keep
		//       the node alive, causing a leak.
		let node = WeakRef::new(self);
		let text_effect = Effect::try_new(move || {
			// Try to get the node
			let node = node.get().or_return()?;

			// And set the text content
			text.with_text(|text| node.set_text_content(text));
		})
		.or_return()?;

		// Then set it
		self.attach_effect(text_effect);
	}
}

/// Extension trait to add reactive text to a node
#[extend::ext(name = NodeWithDynText)]
pub impl<N> N
where
	N: AsRef<web_sys::Node>,
{
	/// Adds dynamic text to this node.
	///
	/// Returns the node, for chaining
	#[track_caller]
	fn with_dyn_text<T>(self, text: T) -> Self
	where
		T: WithDynText + 'static,
	{
		self.as_ref().set_dyn_text(text);
		self
	}
}
