//! Node

// Imports
use {
	crate::{EventTarget, Object, Text, WeakRef, WebError, event_target::EventTargetFields, object::ObjectFields},
	app_error::Context,
	core::mem,
	dynatos_inheritance::{FromFields, Value},
	std::sync::nonpoison::Mutex,
};

/// Node parent
#[derive(Debug)]
pub struct Parent(Option<WeakRef<Node>>);

impl Parent {
	#[must_use]
	pub const fn none() -> Self {
		Self(None)
	}

	#[must_use]
	pub fn new(node: &Node) -> Self {
		Self(Some(WeakRef::new(node)))
	}

	pub fn get(&self) -> Option<Node> {
		self.0.as_ref().and_then(WeakRef::deref)
	}
}

/// Cloning a parent will result in an
/// empty parent, to ensure that clones of
/// a node don't have a parent set.
impl Clone for Parent {
	fn clone(&self) -> Self {
		Self(None)
	}
}

dynatos_inheritance::value! {
	pub struct Node(EventTarget, Object): Send + Sync + Debug {
		parent: Mutex<Parent>,

		node_name: String,
		children: Mutex<Vec<Node>>,
	}
	impl Self {}
}

impl NodeFields {
	pub fn new(node_name: impl Into<String>) -> Self {
		Self {
			parent:    Mutex::new(Parent::none()),
			node_name: node_name.into(),
			children:  Mutex::new(vec![]),
		}
	}
}

impl Node {
	#[must_use]
	pub fn new(node_name: impl Into<String>) -> Self {
		Self::from_fields((
			NodeFields::new(node_name),
			EventTargetFields::default(),
			ObjectFields::default(),
		))
	}

	#[must_use]
	pub fn node_name(&self) -> &str {
		&self.fields().node_name
	}

	pub fn set_text_content(&self, contents: Option<&str>) {
		let text = Text::new(contents.map(str::to_owned));

		let mut children = self.fields().children.lock();
		children.clear();
		children.push(text.into());
	}

	#[must_use]
	pub fn contains(&self, other: Option<&Self>) -> bool {
		let Some(other) = other else { return false };

		let children = self.fields().children.lock();
		children.contains(other) || children.iter().any(|child| child.contains(Some(other)))
	}

	pub fn append_child(&self, child: &Self) -> Result<(), WebError> {
		let mut children = self.fields().children.lock();
		*child.fields().parent.lock() = Parent::new(self);
		children.push(child.clone());
		drop(children);

		Ok(())
	}

	pub fn remove_child(&self, child: &Self) -> Result<(), WebError> {
		let mut children = self.fields().children.lock();
		let idx = children
			.iter()
			.position(|cur_child| cur_child == child)
			.context("Child didn't exist")?;
		children.remove(idx);

		Ok(())
	}

	pub fn replace_child(&self, new_child: &Self, old_child: &Self) -> Result<(), WebError> {
		let mut children = self.fields().children.lock();
		let idx = children
			.iter()
			.position(|child| child == old_child)
			.context("Old child didn't exist")?;

		*new_child.fields().parent.lock() = Parent::new(self);
		*children[idx].fields().parent.lock() = Parent::none();
		children[idx] = new_child.clone();
		Ok(())
	}

	pub fn insert_before(&self, new_child: &Self, child: Option<&Self>) -> Result<(), WebError> {
		match child {
			Some(child) => {
				let mut children = self.fields().children.lock();
				let idx = children
					.iter()
					.position(|cur_child| cur_child == child)
					.context("Reference child didn't exist")?;

				*new_child.fields().parent.lock() = Parent::new(self);
				children.insert(idx, new_child.clone());
				Ok(())
			},
			None => self.append_child(new_child),
		}
	}

	#[must_use]
	pub fn next_sibling(&self) -> Option<Self> {
		let parent = self.fields().parent.lock().clone().get()?;

		let parent_children = parent.fields().children.lock();
		let idx = parent_children
			.iter()
			.position(|child| child == self)
			.expect("We should exist");

		parent_children.get(idx + 1).cloned()
	}

	#[must_use]
	pub fn find_child(&self, mut predicate: impl FnMut(&Self) -> bool) -> Option<Self> {
		let children = self.fields().children.lock();
		children.iter().find(|child| predicate(child)).cloned()
	}

	#[must_use]
	pub fn take_children(&self) -> Vec<Self> {
		mem::take(&mut self.fields().children.lock())
	}
}
