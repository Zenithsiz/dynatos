//! Macros for `dynatos-html`

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

	self::parse_html(&input, input_lit.span(), None)
}

#[proc_macro]
pub fn html_file(input: TokenStream) -> TokenStream {
	let input_file_lit = syn::parse_macro_input!(input as syn::LitStr);
	let input_file = PathBuf::from(input_file_lit.value());
	let input_file = input_file.canonicalize().expect("Unable to canonicalize input file");
	let input = fs::read_to_string(&input_file).expect("Unable to read file");

	self::parse_html(&input, input_file_lit.span(), Some(&input_file))
}

/// Parses html from `input`
fn parse_html(input: &str, span: proc_macro2::Span, dep_file: Option<&Path>) -> TokenStream {
	// Parse the html and parse all the root nodes
	let html = XHtml::parse(input).expect("Unable to parse html");
	let root = html
		.children
		.iter()
		.filter_map(|node| Node::from_html(node, span))
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
			true => quote::quote! { #node },
			false => quote::quote! { web_sys::Node::from(#node) },
		})
		.collect::<Vec<_>>();

	// And finally pack them all into an array, or return the single node
	let root = match &*root {
		[root] => quote::quote! { #root },
		_ => {
			let root = root.into_iter().collect::<Punctuated<_, syn::Token![,]>>();
			quote::quote! { [#root] }
		},
	};

	// Quote the dependency file, if we have one
	let dep = match dep_file {
		Some(dep_file) => {
			let dep_file = dep_file.display().to_string();
			quote::quote! { const _: &[u8] = include_bytes!(#dep_file); }
		},
		None => quote::quote! {},
	};

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
	fn from_html(node: &XHtmlNode, span: proc_macro2::Span) -> Option<Self> {
		let node = match node {
			// If it's an element with an empty name, this is an expression
			XHtmlNode::Element(element) if element.name.as_bytes().is_empty() => {
				let inner = element.inner.expect("Expression cannot be self-closing");
				let expr = syn::parse_str(inner).expect("Unable to parse placeholder");
				Self { ty: NodeTy::Expr, expr }
			},

			// Otherwise, it's a normal element
			XHtmlNode::Element(element) => {
				let name = syn::Ident::new(element.name, span);

				// The element name for building it.
				// Note: The name won't ever conflict with anything else due to it's `mixed_site` span.
				let el = syn::Ident::new("el", proc_macro2::Span::mixed_site());

				// Adds all attributes to the element
				let add_attrs = element
					.attrs
					.iter()
					.map(|(tag, value)| {
						// If the tag name starts with a `:`, the value should be an expression
						match tag.strip_prefix(":") {
							Some(tag) => {
								// Use the tag as the value if none is provided
								let value = value.as_deref().unwrap_or(tag);
								let value = syn::Ident::new(value, span);
								quote::quote! {
									dynatos_html::ElementWithAttr::with_attr(&#el, #tag, #value);
								}
							},
							None => {
								let value = value.unwrap_or_default();
								quote::quote! {
									dynatos_html::ElementWithAttr::with_attr(&#el, #tag, #value);
								}
							},
						}
					})
					.collect::<Vec<_>>();

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
						let child = Self::from_html(child, span)?;
						Some(quote::quote! {
							dynatos_html::NodeAddChildren::add_child(&#el, #child);
						})
					})
					.collect::<Vec<_>>();

				Self {
					ty:   NodeTy::Element,
					expr: syn::parse_quote! {{
						let #el = dynatos_html::html::#name();
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

				Self {
					ty:   NodeTy::Text,
					expr: syn::parse_quote!(
						dynatos_html::text(#text)
					),
				}
			},
			XHtmlNode::Comment(comment) => Self {
				ty:   NodeTy::Comment {},
				expr: syn::parse_quote!(
					dynatos_html::comment(#comment)
				),
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
