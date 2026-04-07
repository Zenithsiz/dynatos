//! Macros for `dynatos-web`

// Imports
use {
	dynatos_web_parser::{XHtml, XHtmlNode},
	proc_macro::TokenStream,
	quote::ToTokens,
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
	#[expect(clippy::too_many_lines, reason = "TODO")]
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
						syn::parse_quote! { dynatos_web::html::#name }
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
								let value = syn::parse_str::<syn::Expr>(value)
									.expect("Unable to parse attribute value as an expression");
								syn::parse_quote! {
									dynatos_web::ElementWithAttr::with_attr(&#el, #tag, &#value);
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
									dynatos_web::EventTargetAddListener::add_event_listener::<dynatos_web::ev!(#tag)>(
										AsRef::<web_sys::EventTarget>::as_ref(&#el),
										#value
									);
								}
							},

							_ => {
								let value = value.unwrap_or_default();
								syn::parse_quote! {
									dynatos_web::ElementWithAttr::with_attr(&#el, #tag, #value);
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

						let expr = match child.ty {
							NodeTy::Element | NodeTy::Text | NodeTy::Comment => syn::parse_quote! {
								dynatos_web::NodeAddChildren::add_children(
									AsRef::<web_sys::Node>::as_ref(&#el),
									#child
								);
							},
							NodeTy::Expr => syn::parse_quote! {
								dynatos_web_reactive::NodeDynChildren::add_dyn_children(
									AsRef::<web_sys::Node>::as_ref(&#el),
									#child
								);
							},
						};

						Some(expr)
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
				// If we're an empty text node, we can simply not add it
				if text.trim().is_empty() {
					return None;
				}

				// Otherwise, process the arguments.
				// Note: If we have any dynamic arguments, we need to create a
				//       closure to evaluate it inside, but we need to evaluate
				//       any statics outside this closure, because they might be
				//       temporaries that we can't capture.
				//       To do this, we first "flatten" every non-dynamic argument,
				//       grouping nearby constants and statics, then we evaluate
				//       those statics outside using `ToString`, and move the evaluated
				//       strings inside of the closure.
				let args = self::split_text_args(text);
				let args = self::flatten_text_args(&args);

				let args_idents = args
					.iter()
					.enumerate()
					.map(|(idx, _)| syn::Ident::new(&format!("_{idx}"), proc_macro2::Span::call_site()))
					.collect::<Vec<_>>();

				let (args_static, args_static_idents) = args
					.iter()
					.zip(&args_idents)
					.filter_map(|(arg, ident)| {
						let expr = arg.try_as_static_arg_ref()?;
						let expr: syn::Expr = syn::parse_quote! { std::string::ToString::to_string(&#expr) };

						Some((expr, ident))
					})
					.unzip::<_, _, Vec<_>, Vec<_>>();

				let args = args
					.iter()
					.zip(&args_idents)
					.map(|(arg, ident)| match arg {
						TextArg::Cons(s) => TextArg::Cons(s),
						TextArg::DynArg(expr) => TextArg::DynArg(expr.clone()),
						TextArg::StaticArg(_) => TextArg::StaticArg(syn::parse_quote! { #ident }),
					})
					.collect::<Vec<_>>();

				let text: syn::Expr = match self::format_text_args(&args, true) {
					TextArg::Cons(s) => syn::parse_quote! {
						dynatos_web::text(#s)
					},
					TextArg::DynArg(expr) => syn::parse_quote! {
						dynatos_web_reactive::NodeWithDynText::with_dyn_text(
							dynatos_web::text(""),
							move || #expr
						)
					},
					TextArg::StaticArg(expr) => syn::parse_quote! {
						dynatos_web::text(&#expr)
					},
				};

				let expr = syn::parse_quote! {
					match ( #( #args_static, )* ) {
						( #( #args_static_idents, )* ) => #text,
					}
				};
				Self { ty: NodeTy::Text, expr }
			},
			XHtmlNode::Comment(comment) => Self {
				ty:   NodeTy::Comment {},
				expr: syn::parse_quote! { dynatos_web::comment(#comment) },
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

#[derive(Clone, Debug)]
#[derive(strum::EnumIs, strum::EnumTryAs)]
enum TextArg<'a> {
	/// Constant
	Cons(&'a str),

	/// Dynamic argument
	DynArg(syn::Expr),

	/// Static argument
	StaticArg(syn::Expr),
}

impl ToTokens for TextArg<'_> {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		match self {
			Self::Cons(s) => s.to_tokens(tokens),
			Self::DynArg(expr) | Self::StaticArg(expr) => expr.to_tokens(tokens),
		}
	}
}

/// Splits a string into constants and arguments
fn split_text_args(mut text: &str) -> Vec<TextArg<'_>> {
	let mut args = vec![];
	while !text.is_empty() {
		// Find the first escape
		#[expect(clippy::mixed_read_write_in_expression, reason = "False positive")]
		let Some(start) = text.find("%{") else {
			if !text.is_empty() {
				args.push(TextArg::Cons(text));
			}
			text = &text[text.len()..];
			continue;
		};

		let Some(end) = text[start..].find("}%") else {
			panic!("Expected `}}%`, found {:?}", &text[start..]);
		};

		if start != 0 {
			args.push(TextArg::Cons(&text[..start]));
		}

		enum ArgKind {
			Dyn,
			Static,
		}
		let arg = &text[start..][2..end];
		let (kind, arg) = match arg {
			arg if let Some(arg) = arg.strip_prefix("static") => (ArgKind::Static, arg),
			arg if let Some(arg) = arg.strip_prefix("dyn") => (ArgKind::Dyn, arg),
			_ => (ArgKind::Dyn, arg),
		};
		let arg = syn::parse_str::<syn::Expr>(arg).expect("Unable to parse argument expression");
		let arg = match kind {
			ArgKind::Dyn => TextArg::DynArg(arg),
			ArgKind::Static => TextArg::StaticArg(arg),
		};
		args.push(arg);

		text = &text[start..][end + 2..];
	}

	args
}

/// Flattens text arguments.
///
/// Joins nearby strings and static arguments.
fn flatten_text_args<'a>(mut args: &[TextArg<'a>]) -> Vec<TextArg<'a>> {
	let mut new_args = vec![];
	while !args.is_empty() {
		let Some(next_dyn_arg) = args.iter().position(TextArg::is_dyn_arg) else {
			new_args.push(self::format_text_args(args, false));
			break;
		};
		new_args.push(self::format_text_args(&args[..next_dyn_arg], false));
		new_args.push(args[next_dyn_arg].clone());
		args = &args[next_dyn_arg + 1..];
	}

	new_args
}

/// Formats text arguments
fn format_text_args<'a>(args: &[TextArg<'a>], to_string: bool) -> TextArg<'a> {
	// If we're just a single argument, just return it as-is.
	if let [arg] = args {
		return arg.clone();
	}

	let fmt = args
		.iter()
		.map(|arg| match arg {
			TextArg::Cons(text) => text,
			TextArg::DynArg(_) | TextArg::StaticArg(_) => "{}",
		})
		.collect::<String>();

	let fmt_args = args
		.iter()
		.filter_map(|arg| match arg {
			TextArg::Cons(_) => None,
			TextArg::DynArg(arg) | TextArg::StaticArg(arg) => Some(arg),
		})
		.collect::<Vec<_>>();

	let fmt = match to_string {
		true => syn::parse_quote! { format!(#fmt, #(#fmt_args),*) },
		false => syn::parse_quote! { format_args!(#fmt, #(#fmt_args),*) },
	};
	match args.iter().any(TextArg::is_dyn_arg) {
		true => TextArg::DynArg(fmt),
		false => TextArg::StaticArg(fmt),
	}
}
