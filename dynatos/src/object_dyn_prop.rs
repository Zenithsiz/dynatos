//! Object reactive property

// Imports
use {
	crate::ObjectAttachEffect,
	core::ops::Deref,
	dynatos_html::{ObjectRemoveProp, ObjectSetProp, WeakRef},
	dynatos_reactive::{Derived, Effect, Memo, Signal, SignalWith, WithDefault},
	dynatos_router::QuerySignal,
	dynatos_util::TryOrReturnExt,
	wasm_bindgen::JsValue,
};

/// Extension trait to add reactive prop to an object
#[extend::ext(name = ObjectDynProp)]
pub impl js_sys::Object {
	/// Adds a dynamic property to this object, where only the value is dynamic.
	#[track_caller]
	fn add_dyn_prop_value<K, V>(&self, key: K, value: V)
	where
		K: AsRef<str> + 'static,
		V: ToDynProp + 'static,
	{
		// The object we're attaching to
		// Note: It's important that we only keep a `WeakRef` to the object.
		//       Otherwise, the object will be keeping us alive, while we keep
		//       the object alive, causing a leak.
		let object = WeakRef::new(self);

		// Create the effect
		let prop_effect = Effect::try_new(move || {
			// Try to get the object
			let object = object.get().or_return()?;

			// Then get the property
			let key = key.as_ref();
			let value = value.to_prop();

			// And finally set/remove the property
			match value {
				Some(value) => {
					object.set_prop(key, value);
				},
				None => {
					object.remove_prop(key);
				},
			}
		})
		.or_return()?;

		// Then set it
		self.attach_effect(prop_effect);
	}
}

/// Extension trait to add reactive prop to an object
#[extend::ext(name = ObjectWithDynProp)]
pub impl<O> O
where
	O: AsRef<js_sys::Object>,
{
	/// Adds a dynamic property to this object, where only the value is dynamic.
	///
	/// Returns the object, for chaining
	#[track_caller]
	fn with_dyn_prop_value<K, V>(self, key: K, value: V) -> Self
	where
		K: AsRef<str> + 'static,
		V: ToDynProp + 'static,
	{
		self.as_ref().add_dyn_prop_value(key, value);
		self
	}
}

/// Trait for values accepted by [`ObjectDynProp`].
///
/// This allows it to work with the following types:
/// - `impl Fn() -> N`
/// - `impl Fn() -> Option<N>`
/// - `N`
/// - `Option<N>`
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

// TODO: Allow impl for `impl SignalGet<Value: WithDynText>`
#[duplicate::duplicate_item(
	Generics Ty;
	[T] [Signal<T> where T: ToDynProp + 'static];
	[T, F] [Derived<T, F> where T: ToDynProp + 'static, F: ?Sized + 'static];
	[T, F] [Memo<T, F> where T: ToDynProp + 'static, F: ?Sized + 'static];
	[S, T] [WithDefault<S, T> where Self: for<'a> SignalWith<Value<'a>: Sized + Deref<Target: ToDynProp>>];
	[T] [QuerySignal<T> where T: ToDynProp + 'static];
)]
impl<Generics> ToDynProp for Ty {
	fn to_prop(&self) -> Option<JsValue> {
		self.with(|prop| prop.to_prop())
	}
}
