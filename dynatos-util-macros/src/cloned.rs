//! `cloned` macro

// Imports
use {
	core::iter,
	proc_macro::TokenStream,
	quote::quote,
	syn::{punctuated::Punctuated, Token},
};

pub fn cloned(attr: TokenStream, input: TokenStream) -> TokenStream {
	let attrs = syn::parse_macro_input!(attr as Attrs);
	let input = syn::parse_macro_input!(input as Input);

	let clones = attrs
		.0
		.into_iter()
		.map(|attr| {
			let ident = attr.ident;
			let expr = match attr.expr {
				Some(expr) => expr,
				None => self::ident_to_expr(ident.clone()),
			};
			syn::parse_quote! {
				let #ident = #expr.clone();
			}
		})
		.collect::<Vec<syn::Stmt>>();

	let output = match input.expr {
		syn::Expr::Let(syn::ExprLet {
			attrs,
			let_token,
			pat,
			eq_token,
			expr,
		}) => quote! {
			#( #attrs )*
			#let_token #pat #eq_token {
				#( #clones )*
				#expr
			};
		},
		expr => quote! {
			{
				#( #clones )*
				#expr
			}
		},
	};

	TokenStream::from(output)
}

/// Input
struct Input {
	expr: syn::Expr,
}

impl syn::parse::Parse for Input {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let expr = input.parse::<syn::Expr>()?;

		// Allow trailing commas / semicolons so we can parse function arguments / statements
		let _trailing_comma = input.parse::<Option<Token![,]>>()?;
		let _trailing_semi = input.parse::<Option<Token![;]>>()?;

		Ok(Self { expr })
	}
}

/// Attributes
struct Attrs(Punctuated<Attr, Token![,]>);

impl syn::parse::Parse for Attrs {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let attrs = Punctuated::parse_terminated(input)?;
		Ok(Self(attrs))
	}
}

/// Attribute
struct Attr {
	ident: syn::Ident,
	expr:  Option<syn::Expr>,
}

impl syn::parse::Parse for Attr {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let ident = input.parse()?;
		let expr = match input.parse::<Option<syn::Token![=]>>()? {
			Some(_eq) => {
				let expr = input.parse::<syn::Expr>()?;
				Some(expr)
			},
			None => None,
		};

		Ok(Self { ident, expr })
	}
}

/// Converts an identifier to an expression
fn ident_to_expr(ident: syn::Ident) -> syn::Expr {
	syn::Expr::Path(syn::ExprPath {
		attrs: Vec::new(),
		qself: None,
		path:  syn::Path {
			leading_colon: None,
			segments:      iter::once(syn::PathSegment {
				ident,
				arguments: syn::PathArguments::None,
			})
			.collect(),
		},
	})
}
