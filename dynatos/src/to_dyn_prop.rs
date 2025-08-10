//! Dynamic properties

use {
	core::ops::Deref,
	dynatos_reactive::{Derived, Memo, Signal, SignalWith, WithDefault, derived::DerivedRun},
	wasm_bindgen::JsValue,
};

/// Values that may be used as possible dynamic properties.
///
/// This allows it to work with the following types:
/// - `i*`, `u*`, `bool`, `String`
/// - `&str`, `&String`
/// - `JsValue`
/// - `impl Fn() -> N`
/// - `Option<N>`
/// - [`Signal`], [`Derived`], [`Memo`], [`WithDefault`]
///
/// Where `N` is a dyn prop.
pub trait ToDynProp {
	/// Gets the current prop
	fn to_prop(&self) -> Option<JsValue>;
}

impl<F, T> ToDynProp for F
where
	F: Fn() -> T,
	T: ToDynProp,
{
	fn to_prop(&self) -> Option<JsValue> {
		self().to_prop()
	}
}

impl<T> ToDynProp for Option<T>
where
	T: ToDynProp,
{
	fn to_prop(&self) -> Option<JsValue> {
		self.as_ref().and_then(T::to_prop)
	}
}

// TODO: Generalize to `impl Into<JsValue>`
#[duplicate::duplicate_item(
	Ty;
	[&'_ str];
	[&'_ String];
	[bool];
	[f32];
	[f64];
	[i128];
	[i16];
	[i32];
	[i64];
	[i8];
	[isize];
	[u128];
	[u16];
	[u32];
	[u64];
	[u8];
	[usize];
)]
impl ToDynProp for Ty {
	fn to_prop(&self) -> Option<JsValue> {
		Some(JsValue::from(*self))
	}
}

#[allow(clippy::allow_attributes, reason = "This only applies in some branches")]
#[allow(clippy::use_self, reason = "We always want to use `JsValue`, not `Ty`")]
#[duplicate::duplicate_item(
	Ty;
	[JsValue];
	[String];
)]
impl ToDynProp for Ty {
	fn to_prop(&self) -> Option<JsValue> {
		Some(JsValue::from(self))
	}
}

#[duplicate::duplicate_item(
	Generics Ty;
	[T] [Signal<T> where T: ToDynProp + 'static];
	[T, F] [Derived<T, F> where T: ToDynProp + 'static, F: ?Sized + DerivedRun<T> + 'static];
	[T, F] [Memo<T, F> where T: ToDynProp + 'static, F: ?Sized + 'static];
	[S, T] [WithDefault<S, T> where Self: for<'a> SignalWith<Value<'a>: Deref<Target: ToDynProp>>];
)]
impl<Generics> ToDynProp for Ty {
	fn to_prop(&self) -> Option<JsValue> {
		#[allow(
			clippy::allow_attributes,
			clippy::redundant_closure_for_method_calls,
			reason = "In some branches it isn't redundant"
		)]
		self.with(|prop| prop.to_prop())
	}
}
