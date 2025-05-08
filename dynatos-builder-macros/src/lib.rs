//! Macros for [`dynatos-builder`]

// Features
#![feature(if_let_guard, try_blocks)]

// Imports
use {
	convert_case::Casing,
	proc_macro::TokenStream,
	quote::quote,
	syn::{punctuated::Punctuated, Token},
};

#[proc_macro_attribute]
pub fn builder(_attr: TokenStream, input: TokenStream) -> TokenStream {
	let input = syn::parse_macro_input!(input as syn::ItemFn);

	// The component and builder name
	let cmpt = &input.sig.ident;
	let builder = quote::format_ident!("{}Builder", cmpt);

	// Visibility for the component and builder
	let cmpt_vis = &input.vis;
	let builder_vis = cmpt_vis;

	// Where bounds for building
	let build_where_bounds = &input.sig.generics.where_clause;

	// Function body
	let build_body = &input.block;

	// Return type
	let ret_ty = &input.sig.output;

	// Function attributes
	let body_attrs = &input.attrs;

	// Async
	let asyncness = &input.sig.asyncness;
	let await_expr = match asyncness {
		Some(_) => quote! { .await },
		None => quote! {},
	};

	// All props
	let props = Prop::parse_all(&input);

	// Builder type params
	let builder_type_params = props
		.iter()
		.map(|prop| self::ident_to_ty_param(self::ident_to_pascal_case(&prop.ident)))
		.collect::<Punctuated<_, Token![,]>>();

	// Builder type arguments
	let builder_type_args = builder_type_params
		.iter()
		.cloned()
		.map(self::ty_param_to_ty)
		.collect::<Punctuated<_, Token![,]>>();

	// Builder default type arguments
	let builder_default_ty_args = props
		.iter()
		.map(|prop| syn::GenericArgument::Type(prop.default_ty.clone().unwrap_or_else(self::missing_prop_ty)))
		.collect::<Punctuated<_, Token![,]>>();

	// Builder props declaration
	let builder_props_decl = props
		.iter()
		.zip(&builder_type_params)
		.map(|(prop, prop_ty)| -> syn::Field {
			let prop_ident = &prop.ident;
			syn::parse_quote! { #prop_ident: #prop_ty }
		})
		.collect::<Punctuated<_, Token![,]>>();

	// Component prop default functions
	let cmpt_prop_default_fns = props
		.iter()
		.filter_map(|prop| {
			let default_ty = prop.default_ty.as_ref()?;

			let fn_name = quote::format_ident!("default_{}", prop.ident);
			let default_value = match &prop.default_value {
				Some(default) => default.clone(),
				None => self::ty_default(default_ty),
			};

			Some(syn::parse_quote! {
				pub fn #fn_name() -> #default_ty {
					#default_value
				}
			})
		})
		.collect::<Vec<syn::ItemFn>>();

	// Builder `build` type params
	let builder_build_ty_params = input
		.sig
		.generics
		.params
		.iter()
		.map(|generic_param| match generic_param {
			syn::GenericParam::Lifetime(_) => panic!("Lifetime arguments aren't supported yet"),
			syn::GenericParam::Type(ty) => ty,
			syn::GenericParam::Const(_) => panic!("Const arguments aren't supported yet"),
		})
		.collect::<Punctuated<_, Token![,]>>();

	// Component `new` method
	let cmpt_new_method: Option<syn::ImplItemFn> = props.iter().all(|prop| prop.default_value.is_some()).then(|| {
		syn::parse_quote! {
			pub #asyncness fn new< #builder_build_ty_params >() #ret_ty
				#build_where_bounds
			{
				Self::builder()
					.build()
					#await_expr
			}
		}
	});

	// Component `from_` methods
	let cmpt_from_methods = props
		.iter()
		.filter(|prop| prop.create_from_fn)
		.map(|prop| {
			let prop_ident = &prop.ident;

			let fn_name = quote::format_ident!("from_{}", prop.ident);
			let prop_ty = &prop.ty;
			syn::parse_quote! {
				pub #asyncness fn #fn_name< #builder_build_ty_params >(
					#prop_ident: #prop_ty,
				) #ret_ty
					#build_where_bounds
				{
					Self::builder()
						.#prop_ident(#prop_ident)
						.build()
						#await_expr
				}
			}
		})
		.collect::<Vec<syn::ImplItemFn>>();

	// Builder methods
	let builder_methods = props
		.iter()
		.enumerate()
		.map(|(prop_idx, prop)| {
			let prop_ident = &prop.ident;

			let fn_name = &prop.ident;
			let mut fn_args = builder_type_args.clone();

			let new_ty_arg = self::ident_to_pascal_case(prop_ident);
			let new_ty_arg = self::ident_to_ty_param(quote::format_ident!("New{}", new_ty_arg));

			fn_args[prop_idx] = self::ty_param_to_ty(new_ty_arg.clone());
			syn::parse_quote! {
				pub fn #fn_name<#new_ty_arg>(
					self,
					#prop_ident: #new_ty_arg,
				) -> #builder< #fn_args > {
					#[allow(clippy::needless_update, reason = "Sometimes, we don't have any prop identifiers")]
					#builder {
						#prop_ident,
						..self
					}
				}
			}
		})
		.collect::<Vec<syn::ImplItemFn>>();

	// Builder `build` type arguments
	let builder_build_type_args = props
		.iter()
		.map(|prop| syn::GenericArgument::Type(prop.ty.clone()))
		.collect::<Punctuated<_, Token![,]>>();

	// Builder prop deconstruct
	// TODO: Find proper type for this?
	let builder_props_deconstruct = props
		.iter()
		.map(|prop| match &prop.pat {
			syn::Pat::Ident(ident) => quote! { #ident },
			pat => {
				let ident = &prop.ident;
				quote! { #ident: #pat }
			},
		})
		.collect::<Punctuated<_, Token![,]>>();

	// Builder prop values
	let builder_prop_values = props
		.iter()
		.map(|prop| -> syn::FieldValue {
			let ident = &prop.ident;
			match &prop.default_value {
				Some(default) => syn::parse_quote! { #ident: #default },
				None => {
					let default = match &prop.default_ty {
						Some(ty) => self::ty_default(ty),
						None => self::ty_default(&self::missing_prop_ty()),
					};

					syn::parse_quote! { #ident: #default }
				},
			}
		})
		.collect::<Punctuated<_, Token![,]>>();

	// Component declaration
	let cmpt_decl: syn::ItemStruct = syn::parse_quote! {
		#cmpt_vis struct #cmpt;
	};

	// Component inherent impl
	let cmpt_inherent_impl: syn::ItemImpl = syn::parse_quote! {
		impl #cmpt {
			/// Creates a builder for this
			pub fn builder() -> #builder < #builder_default_ty_args > {
				#builder::new()
			}

			#( #cmpt_prop_default_fns )*

			#( #cmpt_from_methods )*

			#cmpt_new_method
		}
	};

	// Builder decl
	let builder_decl: syn::ItemStruct = syn::parse_quote! {
		#builder_vis struct #builder< #builder_type_params > {
			#builder_props_decl
		}
	};

	// Builder new impl
	let builder_new_impl: syn::ItemImpl = syn::parse_quote! {
		impl #builder< #builder_default_ty_args > {
			/// Creates a new builder
			pub fn new() -> Self {
				Self {
					#builder_prop_values
				}
			}
		}
	};

	// Builder methods impl
	let builder_methods_impl: syn::ItemImpl = syn::parse_quote! {
		impl< #builder_type_params > #builder< #builder_type_args > {
			#( #builder_methods )*
		}
	};

	// Builder build impl
	let builder_build_impl: syn::ItemImpl = syn::parse_quote! {
		impl< #builder_build_ty_params > #builder < #builder_build_type_args >
			#build_where_bounds
		{
			#( #body_attrs )*
			pub #asyncness fn build(self) #ret_ty {
				let Self {
					#builder_props_deconstruct
				} = self;

				#build_body
			}
		}
	};

	TokenStream::from(quote! {
		#cmpt_decl
		#cmpt_inherent_impl

		#builder_decl
		#builder_new_impl
		#builder_methods_impl
		#builder_build_impl
	})
}

/// A prop
#[derive(Clone, Debug)]
struct Prop {
	/// Prop name
	ident: syn::Ident,

	/// Prop pattern
	pat: syn::Pat,

	/// Prop type
	ty: syn::Type,

	/// Default type
	default_ty: Option<syn::Type>,

	/// Default value
	default_value: Option<syn::Expr>,

	/// If a `from_` function should be created for this component
	create_from_fn: bool,
}

impl Prop {
	/// Parses all props.
	fn parse_all(input: &syn::ItemFn) -> Punctuated<Self, Token![,]> {
		input
			.sig
			.inputs
			.iter()
			.map(|arg| match arg {
				syn::FnArg::Receiver(_) => panic!("Unexpected receiver argument"),
				syn::FnArg::Typed(arg) => Self::parse_single(arg),
			})
			.collect::<Punctuated<_, Token![,]>>()
	}

	/// Parses a prop from a function argument
	fn parse_single(arg: &syn::PatType) -> Self {
		// Get the identifier, if it exists
		let mut prop_ident = match &*arg.pat {
			syn::Pat::Ident(ident) => Some(ident.ident.clone()),
			_ => None,
		};

		// Then search through the attributes
		let mut default_ty = None;
		let mut default_value = None;
		let mut create_from_fn = false;
		for attr in &arg.attrs {
			// Ignore any attributes that aren't `#[prop(...)]`
			let syn::Meta::List(attr) = &attr.meta else {
				continue;
			};

			if !attr.path.is_ident(&quote::format_ident!("prop")) {
				continue;
			}

			// Then parse the inner expression
			let inner = attr
				.parse_args_with(Punctuated::<syn::Meta, Token![,]>::parse_terminated)
				.expect("Unable to parse attribute");

			for inner in inner {
				match inner {
					syn::Meta::Path(path) => {
						let ident = path.get_ident().expect("Expected identifier");
						match ident.to_string().as_str() {
							"from" => create_from_fn = true,
							ident => panic!("Unknown path attribute: {ident:?}"),
						}
					},
					syn::Meta::List(_) => panic!("Unexpected list attribute"),
					syn::Meta::NameValue(name_value) => {
						let ident = name_value.path.get_ident().expect("Expected identifier");
						match ident.to_string().as_str() {
							"name" => match name_value.value {
								syn::Expr::Path(ref path) if let Some(ident) = path.path.get_ident() => {
									prop_ident = Some(ident.clone());
								},
								_ => panic!("Expected prop name to be a single identifier"),
							},
							"default" => match name_value.value {
								syn::Expr::Cast(cast) => {
									default_ty = Some(*cast.ty);
									default_value = Some(*cast.expr);
								},
								_ => panic!("Expected default value to be of the form `<expr> as <ty>`"),
							},
							ident => panic!("Unknown name-value attribute: {ident:?}"),
						}
					},
				}
			}
		}

		if default_ty.is_none() && default_value.is_some() {
			unreachable!("Specified a default value without a type");
		}

		let prop_ident = prop_ident.expect("Props with patterns must specify their name via `#[prop(name = ...)]`");

		Self {
			ident: prop_ident,
			pat: (*arg.pat).clone(),
			ty: (*arg.ty).clone(),
			default_ty,
			default_value,
			create_from_fn,
		}
	}
}

/// Returns `<#T as ::default::Default>::default`
fn ty_default(ty: &syn::Type) -> syn::Expr {
	syn::parse_quote! { <#ty as ::core::default::Default>::default() }
}

/// Converts an identifier to a type
fn ident_to_ty(ident: syn::Ident) -> syn::Type {
	syn::Type::Path(syn::TypePath {
		qself: None,
		path:  ident.into(),
	})
}

/// Converts an identifier to pascal case
fn ident_to_pascal_case(ident: &syn::Ident) -> syn::Ident {
	let new_ident = ident.to_string().to_case(convert_case::Case::Pascal);
	syn::Ident::new(&new_ident, ident.span())
}

/// Converts a type parameter to a type
fn ty_param_to_ty(ty_param: syn::TypeParam) -> syn::Type {
	self::ident_to_ty(ty_param.ident)
}

/// Converts an identifier to a generic parameter
const fn ident_to_ty_param(ident: syn::Ident) -> syn::TypeParam {
	syn::TypeParam {
		attrs: vec![],
		ident,
		colon_token: None,
		bounds: Punctuated::new(),
		eq_token: None,
		default: None,
	}
}

/// Returns the missing prop type
fn missing_prop_ty() -> syn::Type {
	syn::parse_quote! { ::dynatos_builder::MissingProp }
}
