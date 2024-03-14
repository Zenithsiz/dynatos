//! Html wrappers for `dynatos`

// Features
#![feature(decl_macro)]

// Modules
pub mod html;

// Imports
use {itertools::Itertools, wasm_bindgen::JsValue};

/// Extension trait to set the text content in a builder-style.
#[extend::ext_sized(name = NodeWithText)]
pub impl<T> T
where
	T: AsRef<web_sys::Node>,
{
	fn with_text<C>(self, text: C) -> Self
	where
		C: AsTextContent,
	{
		self.as_ref().set_text_content(text.as_text());
		self
	}
}

/// Types that may be used for [`NodeWithText`]
pub trait AsTextContent {
	/// Returns the text
	fn as_text(&self) -> Option<&str>;
}

// Note: We only add a single impl for `Option<_>` to ensure
//       that calling `with_text(None)` works without
//       specifying any type annotations
#[duplicate::duplicate_item(
	Ty body;
	[ &'_ str ] [ Some(self) ];
	[ String ] [ Some(self.as_str()) ];
	[ Option<String> ] [ self.as_deref() ];
)]
impl AsTextContent for Ty {
	fn as_text(&self) -> Option<&str> {
		body
	}
}

/// Extension trait to add children to an node
#[extend::ext_sized(name = NodeAddChildren)]
pub impl<T> T
where
	T: AsRef<web_sys::Node>,
{
	fn add_child<C>(&self, child: C)
	where
		C: AsRef<web_sys::Node>,
	{
		self.add_children([child]);
	}

	fn add_children<C>(&self, children: C)
	where
		C: Children,
	{
		self.try_with_children(children)
			.unwrap_or_else(|err| panic!("Unable to add node children: {err:?}"));
	}

	fn try_add_children<C>(&self, children: C) -> Result<(), JsValue>
	where
		C: Children,
	{
		children.append_all(self.as_ref())
	}
}

/// Extension trait to add children to an node
#[extend::ext_sized(name = NodeWithChildren)]
pub impl<T> T
where
	T: AsRef<web_sys::Node>,
{
	fn with_child<C>(self, child: C) -> Self
	where
		C: AsRef<web_sys::Node>,
	{
		self.with_children([child])
	}

	fn with_children<C>(self, children: C) -> Self
	where
		C: Children,
	{
		self.try_with_children(children)
			.unwrap_or_else(|err| panic!("Unable to add node children: {err:?}"))
	}

	fn try_with_children<C>(self, children: C) -> Result<Self, JsValue>
	where
		C: Children,
	{
		children.append_all(self.as_ref()).map(|()| self)
	}
}

/// Types that may be used for [`NodeWithChildren`]
pub trait Children {
	/// Appends all children in this type
	fn append_all(self, node: &web_sys::Node) -> Result<(), JsValue>;
}

impl Children for () {
	fn append_all(self, _node: &web_sys::Node) -> Result<(), JsValue> {
		Ok(())
	}
}

impl<C> Children for &'_ [C]
where
	C: AsRef<web_sys::Node>,
{
	fn append_all(self, node: &web_sys::Node) -> Result<(), JsValue> {
		for child in self.iter().map(C::as_ref) {
			// If the node already contains the child, warn and refuse to add it.
			if node.contains(Some(child)) {
				tracing::warn!(?child, "Attempted to add a duplicate child");
				continue;
			}

			node.append_child(child.as_ref())?;
		}

		Ok(())
	}
}

impl<C, const N: usize> Children for [C; N]
where
	C: AsRef<web_sys::Node>,
{
	fn append_all(self, node: &web_sys::Node) -> Result<(), JsValue> {
		self.as_slice().append_all(node)
	}
}

impl<C> Children for Vec<C>
where
	C: AsRef<web_sys::Node>,
{
	fn append_all(self, node: &web_sys::Node) -> Result<(), JsValue> {
		self.as_slice().append_all(node)
	}
}

/// Implements `Children` on tuples
macro impl_children_tuple( $( $( $C:ident($idx:tt) ),*; )* ) {
	$(
		impl<$( $C ),*> Children for ($( $C, )*)
		where
			$(
				$C: AsRef<web_sys::Node>,
			)*
		{
			fn append_all(self, node: &web_sys::Node) -> Result<(), JsValue> {
				$({
					// If the node already contains the child, warn and refuse to add it.
					let child = self.$idx.as_ref();
					match node.contains(Some(child)) {
						true => tracing::warn!(?child, "Attempted to add a duplicate child"),
						false => {
							node.append_child(child)?;
						},
					}
				})*

				Ok(())
			}
		}
	)*
}

impl_children_tuple! {
	C0(0);
	C0(0), C1(1);
	C0(0), C1(1), C2(2);
	C0(0), C1(1), C2(2), C3(3);
	C0(0), C1(1), C2(2), C3(3), C4(4);
	C0(0), C1(1), C2(2), C3(3), C4(4), C5(5);
	C0(0), C1(1), C2(2), C3(3), C4(4), C5(5), C6(6);
	C0(0), C1(1), C2(2), C3(3), C4(4), C5(5), C6(6), C7(7);
	C0(0), C1(1), C2(2), C3(3), C4(4), C5(5), C6(6), C7(7), C8(8);
	C0(0), C1(1), C2(2), C3(3), C4(4), C5(5), C6(6), C7(7), C8(8), C9(9);
}

/// Extension trait to add an attribute in a builder-style.
#[extend::ext_sized(name = ElementWithAttr)]
pub impl<T> T
where
	T: AsRef<web_sys::Element>,
{
	fn with_attr<A, V>(self, attr: A, value: V) -> Self
	where
		A: AsRef<str>,
		V: AsRef<str>,
	{
		let attr = attr.as_ref();
		let value = value.as_ref();
		self.try_with_attr(attr, value)
			.unwrap_or_else(|err| panic!("Unable to set element attribute {attr:?} to {value:?}: {err:?}"))
	}

	fn try_with_attr<A, V>(self, attr: A, value: V) -> Result<Self, JsValue>
	where
		A: AsRef<str>,
		V: AsRef<str>,
	{
		self.as_ref()
			.set_attribute(attr.as_ref(), value.as_ref())
			.map(|()| self)
	}
}

/// Extension trait to *append* a class
#[extend::ext_sized(name = ElementAddClass)]
pub impl<T> T
where
	T: AsRef<web_sys::Element>,
{
	fn add_class<C>(&self, class: C)
	where
		C: AsRef<str>,
	{
		self.add_classes([class]);
	}

	fn add_classes<I, C>(&self, classes: I)
	where
		I: IntoIterator<Item = C>,
		C: AsRef<str>,
	{
		// Append all classes to the existing class name and set it.
		// TODO: Not allocate the classes here.
		let classes = classes.into_iter().collect::<Vec<_>>();
		let class_name = self
			.as_ref()
			.class_name()
			.split_whitespace()
			.chain(classes.iter().map(C::as_ref))
			.join(" ");

		self.as_ref().set_class_name(&class_name);
	}
}

/// Extension trait to *append* a class in a builder-style.
#[extend::ext_sized(name = ElementWithClass)]
pub impl<T> T
where
	T: AsRef<web_sys::Element>,
{
	fn with_class<C>(self, class: C) -> Self
	where
		C: AsRef<str>,
	{
		self.as_ref().add_class(class);
		self
	}

	fn with_classes<I, C>(self, classes: I) -> Self
	where
		I: IntoIterator<Item = C>,
		C: AsRef<str>,
	{
		self.as_ref().add_classes(classes);
		self
	}
}
