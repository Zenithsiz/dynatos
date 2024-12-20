//! `XHtml` parser for `dynatos-html`

// Features
#![feature(pattern, try_blocks, try_trait_v2)]

// Imports
use {
	anyhow::Context,
	core::{
		iter,
		ops::{ControlFlow, Try},
		str::pattern::Pattern,
	},
	std::collections::HashMap,
	unicode_xid::UnicodeXID,
};

/// `XHtml`
#[derive(Clone, Debug)]
pub struct XHtml<'a> {
	/// Children
	pub children: Vec<XHtmlNode<'a>>,
}

impl<'a> XHtml<'a> {
	/// Parses an `XHtml` document
	pub fn parse(mut s: &'a str) -> Result<Self, anyhow::Error> {
		// Parse all children until `s` is empty.
		let children = iter::from_fn(|| match s.is_empty() {
			true => None,
			false => Some(XHtmlNode::parse(&mut s)),
		})
		.collect::<Result<Vec<_>, _>>()?;

		Ok(XHtml { children })
	}
}

/// `XHtml` node
#[derive(Clone, Debug)]
pub enum XHtmlNode<'a> {
	/// Element
	Element(XHtmlElement<'a>),

	/// Text
	Text(&'a str),

	/// Comment
	Comment(&'a str),
}

impl<'a> XHtmlNode<'a> {
	/// Parses a node from a string
	fn parse(s: &mut &'a str) -> Result<Self, anyhow::Error> {
		// If it starts with a comment, read until the end of the comment.
		let comment_start = "<!--";
		let comment_end = "-->";
		if s.starts_with(comment_start) {
			let end = s.find(comment_end).context("Expected `-->` after `<!--`")?;
			let comment = &s[comment_start.len()..end];
			*s = &s[end + comment_end.len()..];
			return Ok(Self::Comment(comment));
		}

		// Otherwise, if it starts with `<`, parse an element
		if s.starts_with('<') {
			let el = XHtmlElement::parse(s)?;
			return Ok(Self::Element(el));
		}

		// Finally, just read text until `<` or the end.
		let end = s.find('<').unwrap_or(s.len());
		let text = &s[..end];
		*s = &s[end..];
		Ok(Self::Text(text))
	}
}

/// `XHtml` Element
#[derive(Clone, Debug)]
pub struct XHtmlElement<'a> {
	/// Name
	pub name: &'a str,

	/// Attributes
	pub attrs: HashMap<&'a str, Option<&'a str>>,

	/// Children
	pub children: Vec<XHtmlNode<'a>>,

	/// Inner span
	pub inner: Option<&'a str>,
}

impl<'a> XHtmlElement<'a> {
	/// Parses a node from a string
	fn parse(s: &mut &'a str) -> Result<Self, anyhow::Error> {
		// Parse the element start
		let start = self::parse_element_start(s)?;
		let name = start.name;

		// Then parse the attributes if we weren't empty
		let (attrs, is_self_closing) = match start.is_empty {
			true => (HashMap::new(), false),
			false => {
				self::eat_whitespace(s);
				let res = self::parse_element_attrs(s)?;
				(res.attrs, res.is_self_closing)
			},
		};

		// Then parse all children if we're not self-closing
		let inner_span_start = *s;
		let (children, inner_span_end) = match is_self_closing {
			true => (vec![], None),
			false => {
				let res = self::parse_element_children(s)?;

				anyhow::ensure!(
					name == res.close_name,
					"Expected `{name}`, found `{:?}`, before {s:?}",
					res.close_name
				);

				(res.children, Some(res.inner_span_end))
			},
		};

		// Calculate the inner span
		let inner = inner_span_end.map(|inner_span_end| self::span_from_start_end(inner_span_start, inner_span_end));

		Ok(Self {
			name,
			attrs,
			children,
			inner,
		})
	}
}

/// Eats `pat` from `s`.
///
/// Returns the eaten part
fn eat<'a, P>(s: &mut &'a str, pat: P) -> Option<&'a str>
where
	P: Pattern,
{
	let rest = s.strip_prefix(pat)?;
	let prefix = &s[..s.len() - rest.len()];

	*s = rest;
	Some(prefix)
}

/// Eats all available whitespace
fn eat_whitespace(s: &mut &str) {
	let idx = s.find(|ch: char| !ch.is_whitespace()).unwrap_or(s.len());
	*s = &s[idx..];
}

/// Parses an identifier
fn parse_ident<'a>(s: &mut &'a str) -> Option<&'a str> {
	// TODO: This is technically not compliant, but for our purposes it's
	//       good enough, and we need some extra characters.
	let is_start = |ch: char| ch.is_xid_start() || matches!(ch, ':' | '@');
	let is_cont = |ch: char| ch.is_xid_continue();

	let end = {
		let rest = s.strip_prefix(is_start)?;
		let start = &s[..s.len() - rest.len()];
		match rest.find(|ch| !is_cont(ch)) {
			Some(idx) => start.len() + idx,
			None => s.len(),
		}
	};

	let ident = &s[..end];
	*s = &s[end..];

	Some(ident)
}

/// Parses an attribute value, `"..."` or just `...`
fn parse_attr_value<'a>(s: &mut &'a str) -> Result<&'a str, anyhow::Error> {
	// If it starts with a `"`, go until another `"`
	if self::eat(s, '"').is_some() {
		let end = s.find('"').context("Expected `\"` after `attr=\"...`")?;
		let value = &s[..end];
		*s = &s[end + 1..];
		return Ok(value);
	}

	// Otherwise, read until we find whitespace or `>`
	// TODO: Is reading until whitespace correct here?
	let end = s
		.find(|ch: char| ch.is_whitespace() || matches!(ch, '>'))
		.unwrap_or(s.len());
	let value = &s[..end];
	*s = &s[end..];
	Ok(value)
}

#[derive(Debug)]
struct ParsedElementStart<'a> {
	name:     &'a str,
	is_empty: bool,
}

/// Parses an element start, `<{name}` or `<>`
fn parse_element_start<'a>(s: &mut &'a str) -> Result<ParsedElementStart<'a>, anyhow::Error> {
	anyhow::ensure!(self::eat(s, '<').is_some(), "Expected `<`, found {s:?}");

	self::eat_whitespace(s);
	let (name, is_empty) = match self::eat(s, '>') {
		Some(_) => ("", true),
		None => match self::parse_ident(s) {
			Some(name) => (name, false),
			None => anyhow::bail!("Expected identifier, found {s:?}"),
		},
	};

	Ok(ParsedElementStart { name, is_empty })
}

#[derive(Debug)]
struct ParsedElementAttrs<'a> {
	attrs:           HashMap<&'a str, Option<&'a str>>,
	is_self_closing: bool,
}

/// Parses an element's attributes, a mix of `attr1=value1 attr2=value2` or `attr1 attr2`,
/// followed with `>` or `/>`.
fn parse_element_attrs<'a>(s: &mut &'a str) -> Result<ParsedElementAttrs<'a>, anyhow::Error> {
	let mut is_self_closing = false;
	let attrs = iter::from_fn(|| {
		self::eat_whitespace(s);
		match self::eat(s, '>') {
			Some(_) => None,
			None => {
				if self::eat(s, "/>").is_some() {
					is_self_closing = true;
					return None;
				}
				let Some(attr) = self::parse_ident(s) else {
					return Some(Err(anyhow::anyhow!("Expected identifier, found {s:?}")));
				};
				let value = self::eat(s, '=').map(|_| self::parse_attr_value(s)).transpose();

				Some(try { (attr, value?) })
			},
		}
	})
	.collect::<Result<_, anyhow::Error>>()
	.context("Unable to parse all attributes")?;

	Ok(ParsedElementAttrs { attrs, is_self_closing })
}

#[derive(Debug)]
struct ParsedElementChildren<'a> {
	children:       Vec<XHtmlNode<'a>>,
	inner_span_end: &'a str,
	close_name:     &'a str,
}

/// Parses all children of a tag, along with it's closing tag, `<tag 1><tag 2>...</{name}>`
fn parse_element_children<'a>(s: &mut &'a str) -> Result<ParsedElementChildren<'a>, anyhow::Error> {
	let mut children = vec![];
	let (close_name, inner_span_end) = loop {
		let inner_span_end = *s;
		match self::try_parse(s, self::parse_close_element) {
			Some(name) => {
				break (name, inner_span_end);
			},
			None => children.push(XHtmlNode::parse(s)?),
		}
	};

	Ok(ParsedElementChildren {
		children,
		inner_span_end,
		close_name,
	})
}

/// Parses a closing element, `</{name}>` or `</>`
fn parse_close_element<'a>(s: &mut &'a str) -> Option<&'a str> {
	self::eat(s, '<')?;
	self::eat_whitespace(s);
	self::eat(s, '/')?;
	self::eat_whitespace(s);
	match self::eat(s, '>') {
		Some(_) => Some(""),
		None => {
			let ident = self::parse_ident(s)?;
			self::eat_whitespace(s);
			self::eat(s, '>')?;

			Some(ident)
		},
	}
}

/// Tries to parse with a `&mut &str`.
///
/// On failure, the original string is restored
fn try_parse<'a, F, T>(s: &mut &'a str, f: F) -> T
where
	F: FnOnce(&mut &'a str) -> T,
	T: Try,
{
	let mut s2 = *s;
	match f(&mut s2).branch() {
		ControlFlow::Continue(output) => {
			*s = s2;
			T::from_output(output)
		},
		ControlFlow::Break(res) => T::from_residual(res),
	}
}

/// Returns the span between `start` and `end`
fn span_from_start_end<'a>(start: &'a str, end: &'a str) -> &'a str {
	// |.................|
	//       ^      ^
	//       |      end
	//       start
	//      |.......|: Len: end - start

	&start[..start.len() - end.len()]
}
