//! `value!` macro

// Imports
use {
	convert_case::Casing,
	proc_macro2::Span,
	syn::{parse::Parse, punctuated::Punctuated, token},
};

#[derive(Clone, Copy, Default, Debug)]
struct MainImpls {
	const_:  bool,
	send:    bool,
	sync:    bool,
	default: bool,
}

#[derive(Clone, Copy, Default, Debug)]
struct StorageImpls {
	debug:           bool,
	const_:          bool,
	clone:           bool,
	default_storage: bool,
	default_fields:  bool,
}

#[derive(Clone, Copy, Default, Debug)]
struct VTableImpls {
	const_: bool,
}

#[expect(clippy::too_many_lines, clippy::cognitive_complexity, reason = "TODO")]
pub fn def(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = syn::parse_macro_input!(input as Input);

	let vis = &input.vis;
	let name = &input.ident;
	let parent_tys = &input.parents;

	let fields_name = syn::Ident::new(&format!("{}Fields", input.ident), input.ident.span());
	let storage_name = syn::Ident::new(&format!("{}Storage", input.ident), input.ident.span());

	let methods_name = syn::Ident::new(&format!("{}Methods", input.ident), input.ident.span());
	let vtable_name = syn::Ident::new(&format!("{}VTable", input.ident), input.ident.span());

	let vtable_static_name = syn::Ident::new(
		&format!(
			"{}_VTABLE",
			input.ident.to_string().to_case(convert_case::Case::Constant)
		),
		input.ident.span(),
	);

	let base_ty = syn::parse_quote! { dynatos_inheritance::Base };
	let first_parent_ty = parent_tys.first().unwrap_or(&base_ty);

	let mut main_impls = MainImpls::default();
	let mut storage_impls = StorageImpls::default();
	let mut vtable_impls = VTableImpls::default();
	for trait_ in &input.traits {
		// TODO: Warn if both `DefaultFields` and `Default` are specified.
		match trait_.to_string().as_str() {
			"CloneStorage" => storage_impls.clone = true,
			"DefaultFields" => {
				storage_impls.default_fields = true;
			},
			"Default" => {
				main_impls.default = true;
				storage_impls.default_storage = true;
				storage_impls.default_fields = true;
			},
			"Send" => main_impls.send = true,
			"Sync" => main_impls.sync = true,
			"Debug" => {
				storage_impls.debug = true;
			},
			"Const" => {
				main_impls.const_ = true;
				storage_impls.const_ = true;
				vtable_impls.const_ = true;
			},
			_ =>
				return syn::Error::new(trait_.span(), format!("Unknown trait: {trait_}"))
					.into_compile_error()
					.into(),
		}
	}

	let storage_impls = self::storage(&input, first_parent_ty, &fields_name, &storage_name, storage_impls);
	let vtable_impls = self::vtable(
		&input,
		first_parent_ty,
		&methods_name,
		&vtable_name,
		&vtable_static_name,
		vtable_impls,
	);

	let virtual_methods_impl = input.methods.iter().filter_map(Method::to_virtual_item_impl);

	let override_methods_impl = input.methods.iter().filter_map(Method::to_override_item_impl);

	let virtual_methods = input.methods.iter().filter_map(|method| method.to_virtual_item(name));

	let const_trait = main_impls.const_.then(|| quote::quote! { const });

	let send_impl = main_impls.send.then(|| {
		quote::quote! {
			unsafe impl Send for #name
			where
				#storage_name: Send + Sync
			{}
		}
	});

	let sync_impl = main_impls.send.then(|| {
		quote::quote! {
			unsafe impl Sync for #name
			where
				#storage_name: Send + Sync
			{}
		}
	});

	let default_impl = main_impls.default.then(|| {
		quote::quote! {
			impl Default for #name {
				fn default() -> Self {
					<Self as dynatos_inheritance::Value>::from_storage(
						<Self as dynatos_inheritance::Value>::Storage::default()
					)
				}
			}
		}
	});

	let debug_impl = quote::quote! {
		impl core::fmt::Debug for #name {
			fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
				let mut s = f.debug_struct(stringify!(#name));

				// SAFETY: The storage is valid.
				let base = dynatos_inheritance::ReprTransparent::<dynatos_inheritance::Base>::to_ref(self);
				base.fmt_debug(&mut s);

				s.finish()
			}
		}
	};

	let mut value_for_bounds = Punctuated::<syn::Path, syn::Token![+]>::new();
	if main_impls.send {
		value_for_bounds.push(syn::parse_quote! { Send });
	}
	if main_impls.sync {
		value_for_bounds.push(syn::parse_quote! { Sync });
	}

	quote::quote! {
		#storage_impls
		#vtable_impls

		#[derive(PartialEq, Eq, Clone)]
		#[repr(transparent)]
		#vis struct #name(dynatos_inheritance::Base);

		unsafe impl #const_trait dynatos_inheritance::ReprTransparent<dynatos_inheritance::Base> for #name {}

		#send_impl
		#sync_impl
		#debug_impl
		#default_impl

		impl #name {
			#(
				#override_methods_impl
			)*

			#(
				#virtual_methods_impl
			)*

			#(
				#virtual_methods
			)*
		}

		impl #const_trait dynatos_inheritance::Value for #name {
			type Fields = #fields_name;
			type Methods = #methods_name;

			type Storage = #storage_name;
			type VTable = #vtable_name;

			type Parent = #first_parent_ty;

			const PARENTS: &'static [std::any::TypeId] = &[
				#( std::any::TypeId::of::<#parent_tys>(), )*
			];

			const VTABLE: &'static Self::VTable = &#vtable_static_name;
		}

		impl<T: #value_for_bounds> dynatos_inheritance::ValueFor<T> for #name {}

		impl #const_trait dynatos_inheritance::FromFields for #name {
			type Fields = (
				#fields_name,
				#( <#parent_tys as dynatos_inheritance::Value>::Fields, )*
			);

			fn from_fields(fields: Self::Fields) -> Self {
				let storage = <#storage_name as dynatos_inheritance::StorageFromFields>::from_fields(
					dynatos_inheritance::BaseStorage::new(),
					fields,
				);

				<Self as dynatos_inheritance::Value>::from_storage(storage)
			}
		}

		impl #const_trait dynatos_inheritance::CloneStorage for #name
		where
			#fields_name: Clone,
			#( <#parent_tys as dynatos_inheritance::Value>::Fields: Clone, )*
		{
			fn clone_storage(&self) -> Self {
				let fields = (
					{
						let fields = <Self as dynatos_inheritance::Value>::fields(self);
						<#fields_name as core::clone::Clone>::clone(fields)
					},
					#({
						let parent = <#name as AsRef<#parent_tys>>::as_ref(self);
						let parent_fields = <#parent_tys as dynatos_inheritance::Value>::fields(parent);
						<<#parent_tys as dynatos_inheritance::Value>::Fields as core::clone::Clone>::clone(parent_fields)
					},)*
				);

				<Self as dynatos_inheritance::FromFields>::from_fields(fields)
			}
		}

		impl #const_trait AsRef<dynatos_inheritance::Base> for #name {
			fn as_ref(&self) -> &dynatos_inheritance::Base {
				dynatos_inheritance::ReprTransparent::to_ref(self)
			}
		}

		impl #const_trait From<#name> for dynatos_inheritance::Base {
			fn from(value: #name) -> dynatos_inheritance::Base {
				dynatos_inheritance::ReprTransparent::into_repr(value)
			}
		}

		#(
			impl #const_trait AsRef<#parent_tys> for #name {
				fn as_ref(&self) -> &#parent_tys {
					let ptr = dynatos_inheritance::ReprTransparent::to_ref(self);
					unsafe { <#parent_tys as dynatos_inheritance::ReprTransparent<dynatos_inheritance::Base>>::from_ref(ptr) }
				}
			}

			impl #const_trait From<#name> for #parent_tys {
				fn from(value: #name) -> #parent_tys {
					let ptr = dynatos_inheritance::ReprTransparent::into_repr(value);
					unsafe { <#parent_tys as dynatos_inheritance::ReprTransparent<dynatos_inheritance::Base>>::from_repr(ptr) }
				}
			}
		)*

		impl #const_trait AsRef<#name> for #name {
			fn as_ref(&self) -> &#name {
				self
			}
		}

		impl #const_trait std::ops::Deref for #name {
			type Target = #first_parent_ty;

			fn deref(&self) -> &Self::Target {
				self.as_ref()
			}
		}
	}
	.into()
}

#[expect(clippy::too_many_lines, reason = "TODO")]
fn storage(
	input: &Input,
	first_parent_ty: &syn::Type,
	fields_name: &syn::Ident,
	storage_name: &syn::Ident,
	storage_impls: StorageImpls,
) -> proc_macro2::TokenStream {
	let vis = &input.vis;
	let name = &input.ident;
	let parent_tys = &input.parents;

	let (fields, field_tys) = input
		.fields
		.named
		.iter()
		.map(|field| {
			let field_name = field.ident.as_ref().expect("Expected named field");
			let ty = &field.ty;

			(field_name, ty)
		})
		.unzip::<_, _, Vec<_>, Vec<_>>();

	let storage_from_fields_members = (1..).take(parent_tys.len()).map(syn::Index::from);

	let const_trait = storage_impls.const_.then(|| quote::quote! { const });

	let mut storage_derives = vec![];
	let mut fields_derives = vec![];
	let mut extra_impls = vec![];
	if storage_impls.clone {
		extra_impls.push(quote::quote! {
			impl #const_trait Clone for #fields_name {
				fn clone(&self) -> Self {
					Self {
						#( #fields: self.#fields.clone(), )*
					}
				}
			}

			impl #const_trait Clone for #storage_name {
				fn clone(&self) -> Self {
					Self {
						parent: self.parent.clone(),
						fields: self.fields.clone(),
					}
				}
			}
		});
	}
	if storage_impls.debug {
		storage_derives.push(quote::quote! { Debug });
		fields_derives.push(quote::quote! { Debug });
	}
	if storage_impls.default_storage {
		storage_derives.push(quote::quote! { Default });
	}
	if storage_impls.default_fields {
		fields_derives.push(quote::quote! { Default });
	}

	let debug_fields_impl = storage_impls.debug.then(|| {
		quote::quote! {
			impl dynatos_inheritance::DebugFields for #fields_name {
				fn debug_fields(&self, s: &mut core::fmt::DebugStruct<'_, '_>) {
					#(
						s.field(stringify!(#fields), &self.#fields);
					)*
				}
			}

			impl dynatos_inheritance::DebugFields for #storage_name {
				fn debug_fields(&self, s: &mut core::fmt::DebugStruct<'_, '_>) {
					dynatos_inheritance::DebugFields::debug_fields(&self.parent, s);
					dynatos_inheritance::DebugFields::debug_fields(&self.fields, s);
				}
			}
		}
	});

	quote::quote! {
		#[derive( #( #fields_derives, )* )]
		#vis struct #fields_name {
			#( pub #fields: #field_tys, )*
		}

		#[derive( #( #storage_derives, )* )]
		#[repr(C)]
		#vis struct #storage_name {
			parent: <<#name as dynatos_inheritance::Value>::Parent as dynatos_inheritance::Value>::Storage,
			fields: #fields_name,
		}

		#debug_fields_impl
		#( #extra_impls )*

		impl #const_trait AsRef<#fields_name> for #storage_name {
			fn as_ref(&self) -> &#fields_name {
				&self.fields
			}
		}

		unsafe impl #const_trait dynatos_inheritance::Contains<dynatos_inheritance::BaseStorage> for #storage_name
		where
			<
				<#name as dynatos_inheritance::Value>::Parent
				as dynatos_inheritance::Value
			>::Storage: dynatos_inheritance::Contains<dynatos_inheritance::BaseStorage>
		{}

		#(
			unsafe impl #const_trait dynatos_inheritance::Contains<<#parent_tys as dynatos_inheritance::Value>::Storage> for #storage_name
			{}
		)*

		impl #const_trait dynatos_inheritance::ExtendStorage<#name> for <<#name as dynatos_inheritance::Value>::Parent as dynatos_inheritance::Value>::Storage {
			fn extend_with_fields(self, fields: #fields_name) -> #storage_name {
				#storage_name {
					parent: self,
					fields,
				}
			}
		}

		impl #const_trait dynatos_inheritance::StorageFromFields for #storage_name {
			type Fields = (
				#fields_name,
				#( <#parent_tys as dynatos_inheritance::Value>::Fields, )*
			);

			fn from_fields(base: dynatos_inheritance::BaseStorage, fields: Self::Fields) -> Self {
				Self {
					parent: <<#first_parent_ty as dynatos_inheritance::Value>::Storage as dynatos_inheritance::StorageFromFields>::from_fields(
						base,
						( #( fields.#storage_from_fields_members, )* )
					),
					fields: fields.0,
				}
			}
		}
	}
}

fn vtable(
	input: &Input,
	first_parent_ty: &syn::Type,
	methods_name: &syn::Ident,
	vtable_name: &syn::Ident,
	vtable_static_name: &syn::Ident,
	vtable_impls: VTableImpls,
) -> proc_macro2::TokenStream {
	let vis = &input.vis;
	let name = &input.ident;
	let parent_tys = &input.parents;

	let const_trait = vtable_impls.const_.then(|| quote::quote! { const });

	let virtual_fns = input.methods.iter().filter_map(|method| {
		let Method::Virtual { f, .. } = method else {
			return None;
		};
		let virtual_name = &f.sig.ident;

		let first_arg = syn::parse_quote! { &#name };
		let mut args = Punctuated::<_, syn::Token![,]>::new();
		args.push(&first_arg);
		args.extend(f.sig.inputs.iter().skip(1).map(|arg| match arg {
			syn::FnArg::Receiver(receiver) => &receiver.ty,
			syn::FnArg::Typed(pat_type) => &*pat_type.ty,
		}));

		let ret_ty = &f.sig.output;

		Some(quote::quote! {
			#virtual_name: fn(#args) #ret_ty
		})
	});

	let this_methods: syn::Expr = {
		let methods = input.methods.iter().filter_map(|method| method.to_virtual_field(name));

		syn::parse_quote! {
			#methods_name {
				#( #methods, )*
			}
		}
	};

	let parent_methods = input.parents.iter().map(|parent_ty| {
		let methods = input
			.methods
			.iter()
			.filter_map(|method| method.to_override_field_of(name, parent_ty));

		quote::quote! {
			<#parent_ty as dynatos_inheritance::Value>::Methods {
				#( #methods, )*
			}
		}
	});

	let vtable_from_methods_members = (1..).take(parent_tys.len()).map(syn::Index::from);

	quote::quote! {
		#[derive(Clone, Copy)]
		#vis struct #methods_name {
			#( #virtual_fns, )*
		}

		#[derive(Clone, Copy)]
		#[repr(C)]
		#vis struct #vtable_name {
			parent: <<#name as dynatos_inheritance::Value>::Parent as dynatos_inheritance::Value>::VTable,
			methods: #methods_name,
		}

		impl #const_trait AsRef<#methods_name> for #vtable_name {
			fn as_ref(&self) -> &#methods_name {
				&self.methods
			}
		}

		unsafe impl #const_trait dynatos_inheritance::Contains<dynatos_inheritance::BaseVTable> for #vtable_name
		where
			<
				<#name as dynatos_inheritance::Value>::Parent
				as dynatos_inheritance::Value
			>::VTable: dynatos_inheritance::Contains<dynatos_inheritance::BaseVTable>
		{}

		#(
			unsafe impl #const_trait dynatos_inheritance::Contains<<#parent_tys as dynatos_inheritance::Value>::VTable> for #vtable_name
			{}
		)*

		impl const dynatos_inheritance::VTableFromMethods for #vtable_name {
			type Methods = (
				#methods_name,
				#( <#parent_tys as dynatos_inheritance::Value>::Methods, )*
			);

			fn from_methods(base: dynatos_inheritance::BaseVTable, methods: Self::Methods) -> Self {
				Self {
					parent: <<#first_parent_ty as dynatos_inheritance::Value>::VTable as dynatos_inheritance::VTableFromMethods>::from_methods(
						base,
						( #( methods.#vtable_from_methods_members, )* )
					),
					methods: methods.0,
				}
			}
		}

		static #vtable_static_name: #vtable_name = <#vtable_name as dynatos_inheritance::VTableFromMethods>::from_methods(
			dynatos_inheritance::BaseVTable::new::<#name>(),
			(
				#this_methods,
				#( #parent_methods, )*
			)
		);
	}
}

#[derive(Debug)]
struct Input {
	vis:     syn::Visibility,
	ident:   syn::Ident,
	parents: Vec<syn::Type>,
	traits:  Vec<syn::Ident>,
	fields:  syn::FieldsNamed,
	methods: Vec<Method>,
}

impl Parse for Input {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let vis = input.parse()?;
		let _struct_token = input.parse::<token::Struct>()?;
		let ident = input.parse()?;

		let parents_buffer;
		let _parent_token = syn::parenthesized!(parents_buffer in input);
		let parents = parents_buffer.parse_terminated(syn::Type::parse, syn::Token![,])?;

		let colon = input.parse::<Option<syn::Token![:]>>()?;
		let traits = match colon.is_some() {
			true => Punctuated::<_, syn::Token![+]>::parse_separated_nonempty(input)?,
			false => Punctuated::new(),
		};

		let fields = input.parse()?;

		let _impl_token = input.parse::<token::Impl>()?;
		let _self_token = input.parse::<token::SelfType>()?;

		let items_buffer;
		let _brace_token = syn::braced!(items_buffer in input);

		let mut methods = vec![];
		while !items_buffer.is_empty() {
			methods.push(items_buffer.parse()?);
		}

		Ok(Self {
			vis,
			ident,
			parents: parents.into_iter().collect(),
			traits: traits.into_iter().collect(),
			fields,
			methods,
		})
	}
}

#[derive(Debug)]
#[derive(strum::EnumIs)]
enum Method {
	Virtual {
		_virtual: token::Virtual,
		f:        syn::ItemFn,
	},
	Override {
		_override:    token::Override,
		_paren_token: token::Paren,
		parent_ty:    Box<syn::Type>,
		f:            syn::ItemFn,
	},
}

impl Method {
	fn virtual_impl_name(fn_name: &syn::Ident) -> syn::Ident {
		syn::Ident::new(&format!("{fn_name}_virtual_impl"), fn_name.span())
	}

	fn override_impl_name(fn_name: &syn::Ident) -> syn::Ident {
		syn::Ident::new(&format!("{fn_name}_override_impl"), fn_name.span())
	}

	pub fn to_override_field_of(&self, name: &syn::Ident, expected_parent_ty: &syn::Type) -> Option<syn::FieldValue> {
		let Self::Override {
			_override,
			_paren_token,
			parent_ty,
			f,
		} = self
		else {
			return None;
		};
		if &**parent_ty != expected_parent_ty {
			return None;
		}

		let fn_name = &f.sig.ident;
		let impl_name = Self::override_impl_name(fn_name);

		let (_receiver, arg_idents, arg_tys) = self::args_as_idents(f);

		Some(syn::parse_quote! {
			#fn_name: |this, #( #arg_idents: #arg_tys, )*| {
				let base = dynatos_inheritance::ReprTransparent::<dynatos_inheritance::Base>::to_ref(this);
				let this = unsafe { <#name as dynatos_inheritance::ReprTransparent<dynatos_inheritance::Base>>::from_ref(base) };

				#name::#impl_name(
					this,
					#( #arg_idents, )*
				)
			}
		})
	}

	pub fn to_virtual_field(&self, name: &syn::Ident) -> Option<syn::FieldValue> {
		let Self::Virtual { f, .. } = self else {
			return None;
		};

		let fn_name = &f.sig.ident;
		let impl_name = Self::virtual_impl_name(fn_name);

		let (_receiver, arg_idents, arg_tys) = self::args_as_idents(f);

		Some(syn::parse_quote! {
			#fn_name: |this, #( #arg_idents: #arg_tys, )*| {
				let base = dynatos_inheritance::ReprTransparent::<dynatos_inheritance::Base>::to_ref(this);
				let this = unsafe { <#name as dynatos_inheritance::ReprTransparent<dynatos_inheritance::Base>>::from_ref(base) };

				#name::#impl_name(
					this,
					#( #arg_idents, )*
				)
			}
		})
	}

	pub fn to_override_item_impl(&self) -> Option<syn::ItemFn> {
		let Self::Override { f, .. } = self else {
			return None;
		};

		let mut f = f.clone();
		f.vis = syn::Visibility::Inherited;
		f.sig.ident = Self::override_impl_name(&f.sig.ident);
		Some(f)
	}

	pub fn to_virtual_item_impl(&self) -> Option<syn::ItemFn> {
		let Self::Virtual { f, .. } = self else { return None };
		let mut f = f.clone();
		f.vis = syn::Visibility::Inherited;
		f.sig.ident = Self::virtual_impl_name(&f.sig.ident);
		Some(f)
	}

	pub fn to_virtual_item(&self, name: &syn::Ident) -> Option<syn::ItemFn> {
		let Self::Virtual { f, .. } = self else {
			return None;
		};

		let fn_name = &f.sig.ident;

		let (receiver, arg_idents, arg_tys) = self::args_as_idents(f);
		let ret_ty = &f.sig.output;

		Some(syn::parse_quote! {
			fn #fn_name(#receiver, #( #arg_idents: #arg_tys, )*) #ret_ty {
				let vtable = <#name as dynatos_inheritance::Value>::vtable(&self);
				(vtable.methods.#fn_name)(
					self,
					#( #arg_idents, )*
				)
			}
		})
	}
}

impl Parse for Method {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		match input {
			input if input.peek(token::Virtual) => {
				let virtual_ = input.parse()?;
				let f = input.parse()?;

				Ok(Self::Virtual { _virtual: virtual_, f })
			},
			input if input.peek(token::Override) => {
				let override_ = input.parse()?;
				let parent_buffer;
				let paren_token = syn::parenthesized!(parent_buffer in input);
				let parent_ty = parent_buffer.parse()?;
				let f = input.parse()?;

				Ok(Self::Override {
					_override: override_,
					_paren_token: paren_token,
					parent_ty,
					f,
				})
			},

			_ => Err(syn::Error::new(input.span(), "Expected either `virtual` or `override`")),
		}
	}
}

/// Returns all argument's as identifiers and types
fn args_as_idents(f: &syn::ItemFn) -> (&syn::Receiver, Vec<syn::Ident>, Vec<&syn::Type>) {
	let mut args = f.sig.inputs.iter();

	let first = args.next().expect("Expected at least 1 argument");
	let syn::FnArg::Receiver(first) = first else {
		panic!("Expected the first argument to be a receiver");
	};

	let (idents, tys) = args
		.enumerate()
		.map(|(idx, arg)| {
			let syn::FnArg::Typed(arg) = arg else {
				unreachable!("Receiver must be the first input");
			};

			let ident = syn::Ident::new(&format!("_{idx}"), Span::call_site());

			(ident, &*arg.ty)
		})
		.unzip::<_, _, Vec<_>, Vec<_>>();

	(first, idents, tys)
}
