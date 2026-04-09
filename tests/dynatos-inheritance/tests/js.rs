//! Javascript-like inheritance test

// Features
#![feature(const_trait_impl, more_qualified_paths, trivial_bounds)]

// Imports
use {
	core::cell::RefCell,
	dynatos_inheritance::{CloneStorage, Downcast, FromFields, Value},
	std::collections::HashMap,
};

dynatos_inheritance::value! {
	struct Object(): CloneStorage {}
	impl Self {}
}

dynatos_inheritance::value! {
	struct Node(Object): CloneStorage {
		name: RefCell<String>,
	}

	impl Self {
		virtual fn clone_node(&self) -> Node {
			self.clone_storage()
		}
	}
}

dynatos_inheritance::value! {
	struct Element(Node, Object): CloneStorage {
		attributes: RefCell<HashMap<String, String>>,
	}

	impl Self {
		override(Node) fn clone_node(&self) -> Node {
			self.clone_storage().into()
		}
	}
}

#[test]
fn element_downcast() {
	let element = Element::from_fields((
		ElementFields {
			attributes: RefCell::new(HashMap::new()),
		},
		NodeFields {
			name: RefCell::new("node".to_owned()),
		},
		ObjectFields {},
	));
	element
		.fields()
		.attributes
		.borrow_mut()
		.insert("a".to_owned(), "5".to_owned());

	*(*element).fields().name.borrow_mut() = "node2".to_owned();

	let object = Object::from(element);
	let node = object.clone().downcast::<Node>().expect("Unable to downcast");
	assert_eq!(*node.fields().name.borrow(), "node2");

	{
		let node_clone = node.clone_node();
		assert_eq!(*node_clone.fields().name.borrow(), "node2");
		*node_clone.fields().name.borrow_mut() = "node3".to_owned();
		assert_eq!(*node_clone.fields().name.borrow(), "node3");
		assert_eq!(*node.fields().name.borrow(), "node2");

		let element_clone = node_clone.downcast::<Element>().expect("Unable to downcast");
		assert_eq!(
			element_clone.fields().attributes.borrow().get("a").map(|s| &**s),
			Some("5")
		);
	}

	let element = object.downcast::<Element>().expect("Unable to downcast");
	assert_eq!(element.fields().attributes.borrow().get("a").map(|s| &**s), Some("5"));
}
