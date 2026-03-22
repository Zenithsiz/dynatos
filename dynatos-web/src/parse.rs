//! Html run-time parsing

// Imports
#[expect(unused_imports, reason = "Used by macros")]
use {crate::EventTargetAddListener, core::marker::PhantomData};
use {
	crate::{ElementAddAttr, HTML_NAMESPACE, NodeAddChildren},
	dynatos_web_parser::{XHtml, XHtmlElement, XHtmlNode},
	wasm_bindgen::{JsCast, JsValue},
};

/// Parses html as a single html element
///
/// Ignores any whitespace-only text nodes
/// before and after the parsed element.
///
/// See [`parse`] for details.
pub fn parse_html_element(input: &str, mut environment: impl Environment) -> Result<web_sys::HtmlElement, Error> {
	let html = XHtml::parse(input).map_err(Error::Parse)?;

	let is_whitespace_only = |s: &str| s.chars().all(char::is_whitespace);
	let element = match *html.children.as_slice() {
		[
			XHtmlNode::Text(prefix),
			XHtmlNode::Element(ref node),
			XHtmlNode::Text(suffix),
		] if is_whitespace_only(prefix) && is_whitespace_only(suffix) => node,
		[XHtmlNode::Element(ref node), XHtmlNode::Text(suffix)] if is_whitespace_only(suffix) => node,
		[XHtmlNode::Text(prefix), XHtmlNode::Element(ref node)] if is_whitespace_only(prefix) => node,
		[XHtmlNode::Element(ref node)] => node,
		_ => return Err(Error::SingleElement),
	};

	let element = self::parse_xhtml_element(element, &mut environment)?;
	let element = element.dyn_into().map_err(Error::CastHtmlElement)?;
	Ok(element)
}

/// Parses html at runtime, emitting it as an [`HtmlElement`](web_sys::HtmlElement) list
pub fn parse(input: &str, mut environment: impl Environment) -> Result<Vec<web_sys::Node>, Error> {
	let html = XHtml::parse(input).map_err(Error::Parse)?;
	let children = html
		.children
		.iter()
		.map(|node| self::parse_xhtml_node(node, &mut environment))
		.collect::<Result<Vec<_>, _>>()?;

	Ok(children)
}

fn parse_xhtml_node(node: &XHtmlNode<'_>, environment: &mut impl Environment) -> Result<web_sys::Node, Error> {
	match node {
		XHtmlNode::Element(element) => self::parse_xhtml_element(element, environment),
		XHtmlNode::Text(text) => self::parse_xhtml_text(text, environment),
		XHtmlNode::Comment(comment) => Ok(crate::comment(comment).into()),
	}
}

fn parse_xhtml_text(mut text: &str, environment: &mut impl Environment) -> Result<web_sys::Node, Error> {
	let mut output = String::new();
	while !text.is_empty() {
		// Find the first escape
		let Some(start) = text.find("%{") else {
			output.push_str(text);
			break;
		};

		let Some(end) = text[start..].find("}%") else {
			return Err(Error::TextEscapeEnd);
		};

		output.push_str(&text[..start]);
		text = &text[start..];

		let expr = &text[2..end];
		text = &text[end + 2..];

		let expr = environment.eval_text(expr)?;
		output.push_str(&expr);
	}

	Ok(crate::text(&output).into())
}

fn parse_xhtml_element(
	xhtml_element: &XHtmlElement<'_>,
	environment: &mut impl Environment,
) -> Result<web_sys::Node, Error> {
	// TODO: Cache these
	let window = web_sys::window().expect("Unable to get window");
	let document = window.document().expect("Unable to get document");

	if xhtml_element.name.is_empty() {
		let expr_name = xhtml_element.inner.expect("Empty tags should have a span");
		return environment.eval_node(expr_name);
	}

	let element = match xhtml_element.name.strip_prefix(':') {
		Some(element) => environment.eval_element(element)?,
		None => document
			.create_element_ns(Some(HTML_NAMESPACE), xhtml_element.name)
			.map_err(Error::CreateElement)?,
	};

	#[expect(clippy::iter_over_hash_type, reason = "Attributes are unordered")]
	for (&key, &value) in &xhtml_element.attrs {
		match key {
			key if let Some(key) = key.strip_prefix(':') => {
				let value = environment.eval_attr(key, value)?;
				element.add_attr(key, value);
			},

			key if let Some(event_type) = key.strip_prefix('@') => {
				let value = value.ok_or(Error::EventListenerValue)?;
				environment.eval_ev(&element, event_type, value)?;
			},

			key => element.add_attr(key, value.unwrap_or("")),
		}
	}

	for child in &xhtml_element.children {
		let child = self::parse_xhtml_node(child, environment)?;
		element.add_child(child);
	}

	Ok(element.into())
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("Unable to parse html")]
	Parse(#[source] dynatos_web_parser::Error),

	#[error("Expected a single element")]
	SingleElement,

	#[error("Unable to cast node to `HtmlElement`: {0:?}")]
	CastHtmlElement(web_sys::Node),

	#[error("Expected `}}%` after `%{{`")]
	TextEscapeEnd,

	#[error("Unable to create element: {0:?}")]
	CreateElement(JsValue),

	#[error("Found an event listener attribute without a value")]
	EventListenerValue,

	#[error("Missing element name {element_name:?} in environment")]
	EnvironmentMissingElement { element_name: String },

	#[error("Missing node {expr:?} in environment")]
	EnvironmentMissingNode { expr: String },

	#[error("Missing attribute {attr:?} with value {value:?} in environment")]
	EnvironmentMissingAttr { attr: String, value: Option<String> },

	#[error("Missing event listener {event_type:?} with value {value:?} in environment")]
	EnvironmentMissingEventListener { event_type: String, value: String },

	#[error("Missing text {expr:?} in environment")]
	EnvironmentMissingText { expr: String },
}

impl Error {
	#[must_use]
	pub fn eval_element(element_name: &str) -> Self {
		Self::EnvironmentMissingElement {
			element_name: element_name.to_owned(),
		}
	}

	#[must_use]
	pub fn eval_node(expr: &str) -> Self {
		Self::EnvironmentMissingNode { expr: expr.to_owned() }
	}

	#[must_use]
	pub fn eval_attr(attr: &str, value: Option<&str>) -> Self {
		Self::EnvironmentMissingAttr {
			attr:  attr.to_owned(),
			value: value.map(str::to_owned),
		}
	}

	#[must_use]
	pub fn eval_ev(event_type: &str, value: &str) -> Self {
		Self::EnvironmentMissingEventListener {
			event_type: event_type.to_owned(),
			value:      value.to_owned(),
		}
	}

	#[must_use]
	pub fn eval_text(expr: &str) -> Self {
		Self::EnvironmentMissingText { expr: expr.to_owned() }
	}
}

/// Parsing environment
pub trait Environment {
	/// Evaluates an element
	fn eval_element(&mut self, element_name: &str) -> Result<web_sys::Element, Error> {
		Err(Error::eval_element(element_name))
	}

	/// Evaluates a node
	fn eval_node(&mut self, expr: &str) -> Result<web_sys::Node, Error> {
		Err(Error::eval_node(expr))
	}

	/// Evaluates an attribute
	fn eval_attr(&mut self, attr: &str, value: Option<&str>) -> Result<String, Error> {
		Err(Error::eval_attr(attr, value))
	}

	/// Evaluates an event listener, adding it to `element`
	fn eval_ev(&mut self, _element: &web_sys::Element, event_type: &str, value: &str) -> Result<(), Error> {
		Err(Error::eval_ev(event_type, value))
	}

	/// Evaluates a text
	fn eval_text(&mut self, expr: &str) -> Result<String, Error> {
		Err(Error::eval_text(expr))
	}
}

impl Environment for () {}

/// Creates an environment
pub macro environment(
	$( element { $( $element:ident $(: $element_output:expr)? ),* $(,)? } )?
	$( node    { $( $node   :ident $(: $   node_output:expr)? ),* $(,)? } )?
	$( attr    { $( $attr   :ident $(: $   attr_output:expr)? ),* $(,)? } )?
	$( ev      { $( $ev     :ident $(: $     ev_output:expr)? ),* $(,)? } )?
	$( text    { $( $text   :ident $(: $   text_output:expr)? ),* $(,)? } )?
) {{
	struct E<Element, Node, Attr, Ev, Text> {
		$( $( ${ignore($element)} )* element: Element, )?
		$( $( ${ignore($node   )} )* node   : Node   , )?
		$( $( ${ignore($attr   )} )* attr   : Attr   , )?
		$( $( ${ignore($ev     )} )* ev     : Ev     , )?
		$( $( ${ignore($text   )} )* text   : Text   , )?

		_phantom: PhantomData<(Element, Node, Attr, Ev, Text)>
	}

	impl<Element, Node, Attr, Ev, Text> Environment for E<Element, Node, Attr, Ev, Text>
	where
		$( $( ${ignore($element)} )* Element: Fn(&str) -> Result<web_sys::Element, Error>, )?
		$( $( ${ignore($element)} )* Node: Fn(&str) -> Result<web_sys::Node, Error>, )?
		$( $( ${ignore($attr   )} )* Attr: Fn(&str, Option<&str>) -> Result<String, Error>, )?
		$( $( ${ignore($ev     )} )* Ev: Fn(&web_sys::Element, &str, &str) -> Result<(), Error>, )?
		$( $( ${ignore($text   )} )* Text: Fn(&str) -> Result<String, Error>, )?
	{
		$( fn eval_element(&mut self, element_name: &str) -> Result<web_sys::Element, Error> {
			$( ${ignore($element)} )*
			(self.element)(element_name)
		} )?

		$( fn eval_node(&mut self, expr: &str) -> Result<web_sys::Node, Error> {
			$( ${ignore($node)} )*
			(self.node)(expr)
		} )?

		$( fn eval_attr(&mut self, attr: &str, value: Option<&str>) -> Result<String, Error> {
			$( ${ignore($attr)} )*
			(self.attr)(attr, value)
		} )?

		$( fn eval_ev(&mut self, element: &web_sys::Element, event_type: &str, value: &str) -> Result<(), Error> {
			$( ${ignore($ev)} )*
			(self.ev)(element, event_type, value)
		} )?

		$( fn eval_text(&mut self, expr: &str) -> Result<String, Error> {
			$( ${ignore($text)} )*
			(self.text)(expr)
		} )?
	}

	E::<
		or_else!(! $( $( ${ignore($element)} )* , _ )?),
		or_else!(! $( $( ${ignore($node   )} )* , _ )?),
		or_else!(! $( $( ${ignore($attr   )} )* , _ )?),
		or_else!(! $( $( ${ignore($ev     )} )* , _ )?),
		or_else!(! $( $( ${ignore($text   )} )* , _ )?),
	> {
		$( element: |element_name: &str| match element_name {
			$( stringify!($element) => Ok(or_else!($element $( , $element_output)?)().into()), )*
			_ => Err(Error::eval_element(element_name)),
		},)?

		$( node: |expr: &str| match expr {
			$( stringify!($node) => Ok(or_else!($node $( , $node_output)?)().into()), )*
			_ => Err(Error::eval_node(expr)),
		},)?

		$( attr: |attr: &str, value: Option<&str>| match value.unwrap_or(attr) {
			$( stringify!($attr) => Ok(or_else!($attr $( , $attr_output)?).into()), )*
			_ => Err(Error::eval_attr(attr, value)),
		},)?

		// TODO: Should we give access to the event type?
		$( ev: |element: &web_sys::Element, event_type: &str, value: &str| match value {
			$( stringify!($ev) => {
				element.add_event_listener_untyped(event_type, or_else!($ev $( , $ev_output)?));
				Ok(())
			},)*
			_ => Err(Error::eval_ev(event_type, value)),
		},)?

		$( text: |expr: &str| match expr {
			$( stringify!($text) => Ok(or_else!($text $( , $text_output)?).into()), )*
			_ => Err(Error::eval_text(expr)),
		},)?

		_phantom: PhantomData
	}
}}

#[expect(unused_macros, reason = "False positive")]
macro or_else {
	($default:tt) => { $default },
	($default:tt, $value:tt) => { $value },
}
