//! Element

// Imports
use {
	crate::{
		EventTarget,
		Node,
		Object,
		Text,
		WebError,
		event_target::EventTargetFields,
		node::NodeFields,
		object::ObjectFields,
	},
	app_error::app_error,
	core::fmt,
	zutil_inheritance::{Downcast, FromFields, Value},
	std::{collections::HashMap, sync::nonpoison::Mutex},
};

zutil_inheritance::value! {
	pub struct Element(Node, EventTarget, Object): Send + Sync + Debug {
		tag_name: String,
		class_name: Mutex<String>,
		attrs: Mutex<HashMap<String, String>>,
	}
	impl Self {}
}

impl ElementFields {
	pub fn new(tag_name: impl Into<String>) -> Self {
		Self {
			tag_name:   tag_name.into(),
			class_name: Mutex::new(String::new()),
			attrs:      Mutex::new(HashMap::new()),
		}
	}
}

impl Element {
	#[must_use]
	pub fn new(tag_name: impl Into<String>) -> Self {
		let tag_name = tag_name.into();
		let node_name = tag_name.to_uppercase();

		Self::from_fields((
			ElementFields::new(tag_name),
			NodeFields::new(node_name),
			EventTargetFields::default(),
			ObjectFields::default(),
		))
	}

	#[must_use]
	pub fn class_name(&self) -> String {
		self.get_attribute("class").unwrap_or_default()
	}

	pub fn set_class_name(&self, class_name: &str) {
		self.set_attribute("class", class_name).expect("Should not fail");
	}

	pub fn replace_with_with_node_1(&self, new: &Self) -> Result<(), WebError> {
		let Some(parent) = (**self).fields().parent.lock().clone().get() else {
			return Ok(());
		};

		parent.replace_child(new, self)
	}

	pub fn set_inner_html(&self, _html: &str) {
		// TODO: Support this
		tracing::warn!("Ignoring `Element::set_inner_html` call");
	}

	pub fn get_attribute(&self, attr: &str) -> Result<String, WebError> {
		match self.fields().attrs.lock().get(attr) {
			Some(value) => Ok(value.clone()),
			None => Err(WebError(app_error!("Unknown attribute: {attr:?}"))),
		}
	}

	pub fn set_attribute(&self, attr: &str, value: &str) -> Result<(), WebError> {
		self.fields().attrs.lock().insert(attr.to_owned(), value.to_owned());

		Ok(())
	}

	pub fn remove_attribute(&self, attr: &str) -> Result<(), WebError> {
		self.fields().attrs.lock().remove(attr);

		Ok(())
	}

	fn write_outer_html(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
		let tag_name = &self.fields().tag_name;

		write!(f, "<{tag_name}")?;
		{
			let attrs = self.fields().attrs.lock();
			#[expect(clippy::iter_over_hash_type, reason = "Order doesn't matter")]
			for (key, value) in &*attrs {
				write!(f, " {key}=\"{}\"", value.escape_debug())?;
			}
		}

		let is_void = VOID_ELEMENTS.contains(&&**tag_name);
		let children = (**self).fields().children.lock();
		if is_void && !children.is_empty() {
			tracing::warn!("Void element had children, writing it as a normal element");
		}

		match is_void && children.is_empty() {
			true => {
				drop(children);
				write!(f, "/>")?;
			},
			false => {
				write!(f, ">")?;

				for child in &*children {
					if let Some(child) = child.downcast_ref::<Self>() {
						child.write_outer_html(f)?;
						continue;
					}

					if let Some(child) = child.downcast_ref::<Text>() {
						if let Some(contents) = &child.fields().contents {
							f.pad(contents)?;
						}
						continue;
					}


					tracing::warn!("Ignoring unknown node child: {child:?} ({:?})", child.storage());
				}
				drop(children);

				write!(f, "</{tag_name}>")?;
			},
		}


		Ok(())
	}

	#[must_use]
	pub fn outer_html(&self) -> String {
		fmt::from_fn(|f| self.write_outer_html(f)).to_string()
	}
}

impl fmt::Display for Element {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.write_outer_html(f)
	}
}

const VOID_ELEMENTS: [&str; 14] = [
	"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr",
];
