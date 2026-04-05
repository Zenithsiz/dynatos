//! Html wrappers for `dynatos`

// Features
#![feature(decl_macro, if_let_guard, macro_metavar_expr)]

// Modules
mod event_listener;
pub mod html;
mod object_attach_value;
pub mod parse;

// Exports
pub use self::{
	event_listener::{ElementAddListener, EventListener, EventTargetAddListener, EventTargetWithListener, ev},
	object_attach_value::{ObjectAttachValue, ObjectWithValue},
	parse::{parse, parse_html_element},
};

// Imports
use {
	itertools::Itertools,
	js_sys::Reflect,
	wasm_bindgen::{JsCast, JsValue},
};

/// Parses an html string into an array.
///
/// # Expression
/// This macro supports expressions using an empty tag: `<>this_is_a_variable</>`
///
/// # Output type
/// The type will be `[Node; _]` if there are both `Element`s and `Text` nodes in the html.
///
/// Otherwise, it will be `[Element; _]` / `[Text; _]` / `[Comment; _]` / `[<expr-ty>; _]` if there are
/// only elements, text nodes, comments, or expressions, respectively.
#[doc(inline)]
pub use dynatos_web_macros::html;

/// Parses an html file into an array.
///
/// See [`html!`] for more details
#[doc(inline)]
pub use dynatos_web_macros::html_file;

/// Creates a text node
#[must_use]
pub fn text(data: &str) -> web_sys::Text {
	// TODO: Cache the document in a thread local?
	let window = web_sys::window().expect("Unable to get window");
	let document = window.document().expect("Unable to get document");

	document.create_text_node(data)
}

/// Creates a comment node
#[must_use]
pub fn comment(data: &str) -> web_sys::Comment {
	// TODO: Cache the document in a thread local?
	let window = web_sys::window().expect("Unable to get window");
	let document = window.document().expect("Unable to get document");

	document.create_comment(data)
}

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

/// Extension trait to set the inner html in a builder-style.
#[extend::ext_sized(name = ElementWithInnerHtml)]
pub impl<T> T
where
	T: AsRef<web_sys::Element>,
{
	fn with_inner_html<C>(self, html: C) -> Self
	where
		C: AsTextContent,
	{
		// TODO: Is a default of `""` fine here?
		self.as_ref().set_inner_html(html.as_text().unwrap_or(""));
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
pub impl web_sys::Node {
	fn add_child<C>(&self, child: C)
	where
		C: Child,
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
		C: Child,
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

/// Types that may be used for [`NodeWithChildren`]'s single child methods
pub trait Child {
	/// Appends this child to `node`
	fn append(&self, node: &web_sys::Node) -> Result<(), JsValue>;
}

// TODO: Impl for `impl AsRef<web_sys::Element>` if we can get rid of
//       the conflict due to the blanket impl of `Children`
#[allow(clippy::allow_attributes, reason = "This only applies in some branches")]
#[allow(
	clippy::use_self,
	clippy::useless_asref,
	reason = "We always want to use `web_sys::Element`, not `Ty`"
)]
#[duplicate::duplicate_item(
	Ty;
	[web_sys::Node];
	[web_sys::Text];
	[web_sys::Element];
	[web_sys::HtmlElement];
)]
impl Child for Ty {
	fn append(&self, node: &web_sys::Node) -> Result<(), JsValue> {
		// If the node already contains us, warn and refuse to add it.
		let child = self.as_ref();
		if node.contains(Some(child)) {
			tracing::warn!(?child, "Attempted to add a duplicate child");
			return Ok(());
		}

		node.append_child(child)?;

		Ok(())
	}
}


/// Types that may be used for [`NodeWithChildren`]'s multiple children method
pub trait Children {
	/// Appends all children in this type
	fn append_all(self, node: &web_sys::Node) -> Result<(), JsValue>;
}

impl<C: Child> Children for C {
	fn append_all(self, node: &web_sys::Node) -> Result<(), JsValue> {
		self.append(node)
	}
}

impl Children for () {
	fn append_all(self, _node: &web_sys::Node) -> Result<(), JsValue> {
		Ok(())
	}
}

impl<C> Children for &'_ [C]
where
	C: Child,
{
	fn append_all(self, node: &web_sys::Node) -> Result<(), JsValue> {
		for child in self {
			child.append(node)?;
		}

		Ok(())
	}
}

impl<C, const N: usize> Children for [C; N]
where
	C: Child,
{
	fn append_all(self, node: &web_sys::Node) -> Result<(), JsValue> {
		self.as_slice().append_all(node)
	}
}

impl<C> Children for Vec<C>
where
	C: Child,
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
				$C: Child,
			)*
		{
			fn append_all(self, node: &web_sys::Node) -> Result<(), JsValue> {
				$(
					self.$idx.append(node)?;
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

/// Extension trait to add an attribute
#[extend::ext_sized(name = ElementAddAttr)]
pub impl web_sys::Element {
	fn add_attr<A, V>(&self, attr: A, value: V)
	where
		A: AsRef<str>,
		V: AsRef<str>,
	{
		let attr = attr.as_ref();
		let value = value.as_ref();
		self.try_add_attr(attr, value)
			.unwrap_or_else(|err| panic!("Unable to set element attribute {attr:?} to {value:?}: {err:?}"));
	}

	fn try_add_attr<A, V>(&self, attr: A, value: V) -> Result<(), JsValue>
	where
		A: AsRef<str>,
		V: AsRef<str>,
	{
		self.set_attribute(attr.as_ref(), value.as_ref())
	}
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

/// Extension trait to add a css property in a builder-style.
#[extend::ext_sized(name = HtmlElementWithCssProp)]
pub impl<T> T
where
	T: AsRef<web_sys::HtmlElement>,
{
	fn with_css_prop<A, V>(self, attr: A, value: Option<V>) -> Self
	where
		A: AsRef<str>,
		V: AsRef<str>,
	{
		let attr = attr.as_ref();
		let value = value.as_ref().map(V::as_ref);
		self.try_with_css_prop(attr, value)
			.unwrap_or_else(|err| panic!("Unable to set element css property {attr:?} to {value:?}: {err:?}"))
	}

	fn try_with_css_prop<A, V>(self, attr: A, value: Option<V>) -> Result<Self, JsValue>
	where
		A: AsRef<str>,
		V: AsRef<str>,
	{
		match value {
			Some(value) => self.as_ref().style().set_property(attr.as_ref(), value.as_ref())?,
			None => _ = self.as_ref().style().remove_property(attr.as_ref())?,
		}

		Ok(self)
	}
}

/// Extension trait to *append* a class
#[extend::ext_sized(name = ElementAddClass)]
pub impl web_sys::Element {
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
			.class_name()
			.split_whitespace()
			.chain(classes.iter().map(C::as_ref))
			.join(" ");

		self.set_class_name(&class_name);
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

/// Extension trait to be able to use [`.context`](app_error::AppError::context)
/// on a `Result<T, JsValue>`.
#[cfg(feature = "app-error")]
#[extend::ext(name = JsResultContext)]
pub impl<T> Result<T, JsValue> {
	fn context(self, context: &'static str) -> Result<T, app_error::AppError> {
		self.map_err(|err| app_error::AppError::fmt(format!("{err:?}")).context(context))
	}
}

/// Extension trait to set a property on an object
#[extend::ext(name = ObjectSetProp)]
pub impl js_sys::Object {
	/// Sets the `prop` property of this object to `value`.
	fn set_prop<T>(&self, prop: &str, value: T)
	where
		T: Into<JsValue>,
	{
		let value = value.into();
		Reflect::set(self, &prop.into(), &value)
			.unwrap_or_else(|err| panic!("Unable to set object property {prop:?} to {value:?}: {err:?}"));
	}
}

/// Extension trait to set a property on any type that is an object
#[extend::ext(name = ObjectWithProp)]
pub impl<O> O
where
	O: AsRef<js_sys::Object>,
{
	/// Sets the `prop` property of this object to `value`.
	///
	/// Returns the object, for chaining
	fn with_prop<T>(self, prop: &str, value: T) -> Self
	where
		T: Into<JsValue>,
	{
		self.as_ref().set_prop(prop, value);
		self
	}
}

/// Extension trait to remove a property on an object
#[extend::ext(name = ObjectRemoveProp)]
pub impl js_sys::Object {
	/// Removes the `property` from this object.
	///
	/// Returns if the property existed
	fn remove_prop(&self, property: &str) -> bool {
		Reflect::delete_property(self, &property.into()).expect("Unable to remove object property")
	}
}

/// Error for [`ObjectGet::get`]
#[derive(Clone, Debug)]
pub enum GetError {
	/// Property was missing
	Missing,

	/// Property was the wrong type
	WrongType(JsValue),
}

/// Extension trait to get a property of an object
#[extend::ext(name = ObjectGet)]
pub impl js_sys::Object {
	// TODO: Differentiate between missing value and wrong type?
	fn get<T>(&self, property: &str) -> Result<T, GetError>
	where
		T: JsCast,
	{
		// Note: This returning `Err` should only happen if `self` isn't an object,
		//       which we guarantee, so no errors can occur.
		let value = Reflect::get(self, &property.into()).expect("Unable to get object property");
		if value.is_undefined() {
			return Err(GetError::Missing);
		}

		value.dyn_into().map_err(GetError::WrongType)
	}
}

/// Html namespace
const HTML_NAMESPACE: &str = "http://www.w3.org/1999/xhtml";
