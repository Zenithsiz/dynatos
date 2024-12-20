//! Macros for `dynatos-html`

// Features
#![feature(if_let_guard)]

// Imports
use {
	dynatos_html_parser::{XHtml, XHtmlNode},
	proc_macro::TokenStream,
	std::{
		fs,
		path::{Path, PathBuf},
	},
	syn::punctuated::Punctuated,
};

#[proc_macro]
pub fn html(input: TokenStream) -> TokenStream {
	let input_lit = syn::parse_macro_input!(input as syn::LitStr);
	let input = input_lit.value();

	self::parse_html(&input, None)
}

#[proc_macro]
pub fn html_file(input: TokenStream) -> TokenStream {
	let input_file_lit = syn::parse_macro_input!(input as syn::LitStr);
	let input_file = PathBuf::from(input_file_lit.value());
	let input_file = input_file.canonicalize().expect("Unable to canonicalize input file");
	let input = fs::read_to_string(&input_file).expect("Unable to read file");

	self::parse_html(&input, Some(&input_file))
}

/// Parses html from `input`
fn parse_html(input: &str, dep_file: Option<&Path>) -> TokenStream {
	// Parse the html and parse all the root nodes
	let html = XHtml::parse(input).expect("Unable to parse html");
	let root = html
		.children
		.iter()
		.filter_map(|node| Node::from_html(node))
		.collect::<Vec<_>>();

	// Check if all nodes have the same type.
	// Note: This is so we can avoid the cast to `Node` if we can avoid it, and
	//       instead keep all the root nodes as their own types.
	let root_ty_all_eq = root
		.iter()
		.try_fold(None, |cur_ty, node| match cur_ty {
			Some(cur_ty) => match cur_ty == node.ty {
				true => Some(Some(node.ty)),
				false => None,
			},
			None => Some(Some(node.ty)),
		})
		.flatten()
		.is_some();

	// Then quote all the root nodes
	let root = root
		.into_iter()
		.map(|node| match root_ty_all_eq {
			true => syn::parse_quote! { #node },
			false => syn::parse_quote! { web_sys::Node::from(#node) },
		})
		.collect::<Vec<syn::Expr>>();

	// And finally pack them all into an array, or return the single node
	let root: syn::Expr = match &*root {
		[root] => syn::parse_quote! { #root },
		_ => {
			let root = root.into_iter().collect::<Punctuated<_, syn::Token![,]>>();
			syn::parse_quote! { [#root] }
		},
	};

	// Quote the dependency file, if we have one
	let dep: Option<syn::Stmt> = dep_file.map(|dep_file| {
		let dep_file = dep_file.display().to_string();
		syn::parse_quote! { const _: &[u8] = include_bytes!(#dep_file); }
	});

	TokenStream::from(quote::quote! {{
		#dep
		#root
	}})
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum NodeTy {
	/// An html element
	Element,

	/// A text element
	Text,

	/// A comment
	Comment,

	/// A generic expression
	Expr,
}

#[derive(Clone, Debug)]
struct Node {
	ty:   NodeTy,
	expr: syn::Expr,
}

impl Node {
	/// Parses a node `node` from an html node.
	///
	/// Returns `None` is `node` is an empty text element.
	fn from_html(node: &XHtmlNode) -> Option<Self> {
		let node = match node {
			// If it's an element with an empty name, this is an expression
			XHtmlNode::Element(element) if element.name.is_empty() => {
				let inner = element.inner.expect("Expression cannot be self-closing");
				let expr = syn::parse_str(inner).expect("Unable to parse placeholder");
				Self { ty: NodeTy::Expr, expr }
			},

			// Otherwise, it's a normal element
			XHtmlNode::Element(element) => {
				// If the name starts with a `:`, use an expression for the constructor
				let constructor: syn::Expr = match element.name.strip_prefix(':') {
					Some(expr) => {
						let expr =
							syn::parse_str::<syn::Expr>(expr).expect("Unable to parse tag name as an expression");
						syn::parse_quote! { #expr }
					},
					None => {
						let name = syn::parse_str::<syn::Ident>(element.name)
							.expect("Unable to parse tag name as an identifier");
						syn::parse_quote! { dynatos_html::html::#name }
					},
				};

				// The element name for building it.
				// Note: The name won't ever conflict with anything else due to it's `mixed_site` span.
				let el = syn::Ident::new("el", proc_macro2::Span::mixed_site());

				// Adds all attributes to the element
				let add_attrs = element
					.attrs
					.iter()
					.map(|(tag, value)| {
						match tag {
							// If the tag name starts with a `:`, the value should be an expression
							tag if let Some(tag) = tag.strip_prefix(":") => {
								// Use the tag as the value if none is provided
								let value = value.as_deref().unwrap_or(tag);
								let value = syn::parse_str::<syn::Ident>(value)
									.expect("Unable to parse attribute value as an identifier");
								syn::parse_quote! {
									dynatos_html::ElementWithAttr::with_attr(&#el, #tag, #value);
								}
							},

							// If the tag name starts with a `@`, the value should be an event listener
							tag if let Some(tag) = tag.strip_prefix("@") => {
								// Use the tag as the event type
								let tag = syn::parse_str::<syn::Ident>(tag)
									.expect("Unable to parse attribute name as an identifier");

								// Use the value as the function handler
								let value = value.as_deref().expect("Event listener needs a value");
								let value =
									syn::parse_str::<syn::Expr>(value).expect("Unable to parse event listener value");

								syn::parse_quote! {
									dynatos_util::EventTargetAddListener::add_event_listener::<dynatos_util::ev::#tag>(&#el, #value);
								}
							},

							_ => {
								let value = value.unwrap_or_default();
								syn::parse_quote! {
									dynatos_html::ElementWithAttr::with_attr(&#el, #tag, #value);
								}
							},
						}
					})
					.collect::<Vec<syn::Stmt>>();

				// Adds all children to the element
				//
				// Note: Unlike at the top-level, here we don't care to cast
				//       all children to the type, as we'll be adding them separately.
				// TODO: If we only contain text nodes, should we collect them all and
				//       use `set_text_content` instead?
				let add_children = element
					.children
					.iter()
					.filter_map(|child| {
						let child = Self::from_html(child)?;
						Some(syn::parse_quote! {
							dynatos_html::NodeAddChildren::add_child(&#el, #child);
						})
					})
					.collect::<Vec<syn::Stmt>>();

				Self {
					ty:   NodeTy::Element,
					expr: syn::parse_quote! {{
						let #el = #constructor();
						#(#add_attrs)*
						#(#add_children)*
						#el
					}},
				}
			},
			XHtmlNode::Text(text) => {
				// If we're an empty text node, return `None`.
				if text.trim().is_empty() {
					return None;
				}

				let args = self::split_text_args(text);

				// If we have just a single constant argument, return a simple version
				if let [TextArg::Cons(text)] = &*args {
					return Some(Self {
						ty:   NodeTy::Text,
						expr: syn::parse_quote! { dynatos_html::text(#text) },
					});
				};

				// Otherwise, we'll format a string with dynamic text
				let fmt = args
					.iter()
					.map(|arg| match arg {
						TextArg::Cons(text) => text,
						TextArg::Argument(_) => "{}",
					})
					.collect::<String>();

				let args = args
					.into_iter()
					.filter_map(|arg| match arg {
						TextArg::Cons(_) => None,
						TextArg::Argument(arg) => {
							let arg = syn::parse_str::<syn::Expr>(arg).expect("Unable to parse argument expression");
							Some(arg)
						},
					})
					.collect::<Vec<_>>();

				Self {
					ty:   NodeTy::Text,
					expr: syn::parse_quote! { dynatos::NodeWithDynText::with_dyn_text(
						dynatos_html::text(""),
						move || format!(#fmt, #(#args),*)
					)},
				}
			},
			XHtmlNode::Comment(comment) => Self {
				ty:   NodeTy::Comment {},
				expr: syn::parse_quote! { dynatos_html::comment(#comment) },
			},
		};

		Some(node)
	}
}

impl quote::ToTokens for Node {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		self.expr.to_tokens(tokens);
	}
}

enum TextArg<'a> {
	/// Constant
	Cons(&'a str),

	/// Argument
	Argument(&'a str),
}

/// Splits a string into constants and arguments
fn split_text_args(mut text: &str) -> Vec<TextArg> {
	let mut args = vec![];
	while !text.is_empty() {
		// Find the first escape
		#[expect(clippy::mixed_read_write_in_expression, reason = "False positive")]
		let Some(start) = text.find("%{") else {
			args.push(TextArg::Cons(text));
			text = &text[text.len()..];
			continue;
		};

		let Some(end) = text[start..].find("}%") else {
			panic!("Expected `}}%`, found {:?}", &text[start..]);
		};

		args.push(TextArg::Cons(&text[..start]));
		args.push(TextArg::Argument(&text[start..][2..end]));
		text = &text[start..][end + 2..];
	}

	args
}
