//! Html wrappers for `dynatos`

// Features
#![feature(decl_macro, macro_metavar_expr, trivial_bounds)]
#![cfg_attr(feature = "ssr", feature(nonpoison_mutex, sync_nonpoison))]
#![cfg_attr(feature = "csr", feature(unsize))]

// Modules
mod as_parent;
mod ctx;
mod event_listener;
pub mod html;
mod object_attach_value;
pub mod parse;
mod util;

// Exports
pub use self::{
	ctx::DynatosWebCtx,
	event_listener::{ElementAddListener, EventListener, EventTargetAddListener, EventTargetWithListener, ev},
	object_attach_value::{ObjectAttachValue, ObjectWithValue},
	parse::{parse, parse_html_element},
};

/// Types for the dynatos web interface.
///
/// With the `csr` feature, this will be an export of
/// types within `wasm_bindgen`, `js_sys` and `web_sys`.
///
/// With the `ssr` feature, this will be an export of
/// types within `dynatos_web_ssr`.
pub mod types {
	pub use dynatos_web_types::*;
}

// Imports
use {
	self::as_parent::AsParent,
	itertools::Itertools,
	types::{Comment, Element, HtmlElement, JsCast, JsValue, Node, Object, Text, WebError, cfg_ssr_expr},
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
pub fn text(ctx: &DynatosWebCtx, data: &str) -> Text {
	ctx.document().create_text_node(data)
}

/// Creates a comment node
#[must_use]
pub fn comment(ctx: &DynatosWebCtx, data: &str) -> Comment {
	ctx.document().create_comment(data)
}

/// Extension trait to set the text content in a builder-style.
#[extend::ext_sized(name = NodeWithText)]
pub impl<T> T
where
	T: AsRef<Node>,
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
	T: AsRef<Element>,
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
pub impl Node {
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
		self.try_add_children(children)
			.unwrap_or_else(|err| panic!("Unable to add node children: {err:?}"));
	}

	fn try_add_children<C>(&self, children: C) -> Result<(), WebError>
	where
		C: Children,
	{
		children.append_all(self)
	}
}

/// Extension trait to add children to an node
#[extend::ext_sized(name = NodeWithChildren)]
pub impl<T> T
where
	T: AsRef<Node>,
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

	fn try_with_children<C>(self, children: C) -> Result<Self, WebError>
	where
		C: Children,
	{
		children.append_all(self.as_ref()).map(|()| self)
	}
}

/// Types that may be used for [`NodeWithChildren`]'s single child methods
pub trait Child {
	/// Appends this child to `node`
	fn append(&self, node: &Node) -> Result<(), WebError>;
}

impl<T: AsParent<Node>> Child for T {
	fn append(&self, node: &Node) -> Result<(), WebError> {
		// If the node already contains us, warn and refuse to add it.
		let child = self.as_parent();
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
	fn append_all(self, node: &Node) -> Result<(), WebError>;
}

impl<C: Child> Children for C {
	fn append_all(self, node: &Node) -> Result<(), WebError> {
		self.append(node)
	}
}

impl Children for () {
	fn append_all(self, _node: &Node) -> Result<(), WebError> {
		Ok(())
	}
}

impl<C> Children for &'_ [C]
where
	C: Child,
{
	fn append_all(self, node: &Node) -> Result<(), WebError> {
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
	fn append_all(self, node: &Node) -> Result<(), WebError> {
		self.as_slice().append_all(node)
	}
}

impl<C> Children for Vec<C>
where
	C: Child,
{
	fn append_all(self, node: &Node) -> Result<(), WebError> {
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
			fn append_all(self, node: &Node) -> Result<(), WebError> {
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
pub impl Element {
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

	fn try_add_attr<A, V>(&self, attr: A, value: V) -> Result<(), WebError>
	where
		A: AsRef<str>,
		V: AsRef<str>,
	{
		self.set_attribute(attr.as_ref(), value.as_ref())?;
		Ok(())
	}
}

/// Extension trait to add an attribute in a builder-style.
#[extend::ext_sized(name = ElementWithAttr)]
pub impl<T> T
where
	T: AsRef<Element>,
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

	fn try_with_attr<A, V>(self, attr: A, value: V) -> Result<Self, WebError>
	where
		A: AsRef<str>,
		V: AsRef<str>,
	{
		self.as_ref().set_attribute(attr.as_ref(), value.as_ref())?;
		Ok(self)
	}
}

/// Extension trait to set a css property
#[extend::ext_sized(name = HtmlElementSetCssProp)]
pub impl HtmlElement {
	fn set_css_prop<A, V>(&self, attr: A, value: Option<V>)
	where
		A: AsRef<str>,
		V: AsRef<str>,
	{
		let attr = attr.as_ref();
		let value = value.as_ref().map(V::as_ref);
		self.try_set_css_prop(attr, value)
			.unwrap_or_else(|err| panic!("Unable to set element css property {attr:?} to {value:?}: {err:?}"));
	}

	fn try_set_css_prop<A, V>(&self, attr: A, value: Option<V>) -> Result<(), WebError>
	where
		A: AsRef<str>,
		V: AsRef<str>,
	{
		match value {
			Some(value) => self.style().set_property(attr.as_ref(), value.as_ref())?,
			None => _ = self.style().remove_property(attr.as_ref())?,
		}

		Ok(())
	}
}

/// Extension trait to set a css property in a builder-style.
#[extend::ext_sized(name = HtmlElementWithCssProp)]
pub impl<T> T
where
	T: AsRef<HtmlElement>,
{
	fn with_css_prop<A, V>(self, attr: A, value: Option<V>) -> Self
	where
		A: AsRef<str>,
		V: AsRef<str>,
	{
		self.as_ref().set_css_prop(attr, value);
		self
	}

	fn try_with_css_prop<A, V>(self, attr: A, value: Option<V>) -> Result<Self, WebError>
	where
		A: AsRef<str>,
		V: AsRef<str>,
	{
		self.as_ref().try_set_css_prop(attr, value)?;
		Ok(self)
	}
}

/// Extension trait to *append* a class
#[extend::ext_sized(name = ElementAddClass)]
pub impl Element {
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
	T: AsRef<Element>,
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
/// on a `Result<T, impl AsRef<WebError>>`.
#[extend::ext(name = JsResultContext)]
pub impl<T, E: AsRef<WebError>> Result<T, E> {
	fn context(self, context: &'static str) -> Result<T, app_error::AppError> {
		cfg_ssr_expr!(
			ssr = self.map_err(|err| err.as_ref().0.context(context)),
			csr = self.map_err(|err| app_error::AppError::fmt(format!("{:?}", err.as_ref())).context(context)),
		)
	}
}

/// Extension trait to set a property on an object
#[extend::ext(name = ObjectSetProp)]
pub impl Object {
	/// Sets the `prop` property of this object to `value`.
	fn set_prop<T>(&self, prop: &str, value: T)
	where
		T: Into<JsValue>,
	{
		cfg_ssr_expr!(
			ssr = Object::set_prop_inner(self, prop, value),
			csr = {
				let value = value.into();
				js_sys::Reflect::set(self, &prop.into(), &value)
					.unwrap_or_else(|err| panic!("Unable to set object property {prop:?} to {value:?}: {err:?}"));
			}
		);
	}
}

/// Extension trait to set a property on any type that is an object
#[extend::ext(name = ObjectWithProp)]
pub impl<O> O
where
	O: AsRef<Object>,
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
pub impl Object {
	/// Removes the `property` from this object.
	///
	/// Returns if the property existed
	fn remove_prop(&self, prop: &str) -> bool {
		cfg_ssr_expr!(
			ssr = Object::remove_prop_inner(self, prop).is_some(),
			csr = js_sys::Reflect::delete_property(self, &prop.into()).expect("Unable to remove object property")
		)
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
pub impl Object {
	fn get<T>(&self, prop: &str) -> Result<T, GetError>
	where
		T: JsCast,
	{
		cfg_ssr_expr!(
			ssr = self.get_prop_inner(prop).map_err(|err| match err {
				dynatos_web_ssr::object::GetError::Missing => GetError::Missing,
				dynatos_web_ssr::object::GetError::WrongType(ty) => GetError::WrongType(ty),
			}),
			csr = {
				// Note: This returning `Err` should only happen if `self` isn't an object,
				//       which we guarantee, so no errors can occur.
				let value = js_sys::Reflect::get(self, &prop.into()).expect("Unable to get object property");

				if value.is_undefined() {
					return Err(GetError::Missing);
				}

				value.dyn_into().map_err(GetError::WrongType)
			}
		)
	}
}

#[extend::ext(name = WindowAnimationFrame)]
#[cfg(feature = "csr")]
pub impl web_sys::Window {
	/// Requests an animation frame with `f` as the callback
	fn request_anim_frame(&self, f: impl FnOnce(f64) + 'static) -> i32 {
		let mut f = Some(f);
		// TODO: Nothing in MDN says this can fail, can it?
		self.request_animation_frame(&util::csr::js_fn(move |time| {
			f.take().expect("Animation frame called twice")(time);
		}))
		.expect("Unable to request animation frame")
	}

	/// Requests several animation frames with `f` as the callback after `len` frames.
	///
	/// # Length
	/// Calling this with `len == 0` will have the same effect as [`request_anim_frame`](WindowAnimationFrame::request_anim_frame).
	fn request_anim_frame_after(&self, len: usize, f: impl FnOnce() + 'static) {
		fn inner(window: &web_sys::Window, cur_frame: usize, len: usize, f: impl FnOnce() + 'static) {
			let f = {
				let window = window.clone();
				move |_| match cur_frame == len {
					true => f(),
					false => inner(&window, cur_frame + 1, len, f),
				}
			};
			window.request_anim_frame(f);
		}
		inner(self, 0, len, f);
	}
}

/// Html namespace
const HTML_NAMESPACE: &str = "http://www.w3.org/1999/xhtml";
