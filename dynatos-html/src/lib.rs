//! Html wrappers for [`dynatos`]

// Features
#![feature(decl_macro)]

// Modules
pub mod html;

// Imports
use wasm_bindgen::JsValue;

/// Extension trait to set the text content in a builder-style.
#[extend::ext_sized(name = NodeWithTextContent)]
pub impl<T> T
where
	T: AsRef<web_sys::Node>,
{
	fn with_text_content<C>(self, text: C) -> Self
	where
		C: AsTextContent,
	{
		self.as_ref().set_text_content(text.as_text_content());
		self
	}
}

/// Types that may be used for [`ElementWithTextContent`]
pub trait AsTextContent {
	/// Returns the text content
	fn as_text_content(&self) -> Option<&str>;
}

// Note: We only add a single impl for `Option<_>` to ensure
//       that calling `with_text_content(None)` works without
//       specifying any type annotations
#[duplicate::duplicate_item(
	Ty body;
	[ &'_ str ] [ Some(self) ];
	[ String ] [ Some(self.as_str()) ];
	[ Option<String> ] [ self.as_deref() ];
)]
impl AsTextContent for Ty {
	fn as_text_content(&self) -> Option<&str> {
		body
	}
}

/// Extension trait to add children to an node
#[extend::ext_sized(name = NodeWithChildren)]
pub impl<T> T
where
	T: AsRef<web_sys::Node>,
{
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

/// Types that may be used for [`ElementWithTextContent`]
pub trait Children {
	/// Appends all children in this type
	fn append_all(self, node: &web_sys::Node) -> Result<(), JsValue>;
}

impl Children for () {
	fn append_all(self, _node: &web_sys::Node) -> Result<(), JsValue> {
		Ok(())
	}
}

impl<C, const N: usize> Children for [C; N]
where
	C: AsRef<web_sys::Node>,
{
	fn append_all(self, node: &web_sys::Node) -> Result<(), JsValue> {
		for child in self {
			node.append_child(child.as_ref())?;
		}

		Ok(())
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
				$(
					node.append_child(self.$idx.as_ref())?;
				)*

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
