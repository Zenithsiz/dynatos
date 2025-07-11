//! Element reactive attribute

// Imports
use {
	crate::ObjectAttachEffect,
	core::ops::Deref,
	dynatos_html::WeakRef,
	dynatos_reactive::{derived::DerivedRun, Derived, Effect, Memo, Signal, SignalWith, WithDefault},
	dynatos_util::TryOrReturnExt,
};

/// Extension trait to add reactive attribute to an element
#[extend::ext(name = ElementDynAttr)]
pub impl web_sys::Element {
	/// Adds a dynamic attribute to this element
	#[track_caller]
	fn set_dyn_attr<K, V>(&self, key: K, value: V)
	where
		K: AsRef<str> + 'static,
		V: WithDynAttr + 'static,
	{
		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the element.
		//       Otherwise, the element will be keeping us alive, while we keep
		//       the element alive, causing a leak.
		let element = WeakRef::new(self);
		let attr_effect = Effect::try_new(move || {
			// Try to get the element
			let element = element.get().or_return()?;

			// And set the attribute
			let key = key.as_ref();
			value.with_attr(|value| match value {
				Some(value) => element
					.set_attribute(key, value)
					.unwrap_or_else(|err| panic!("Unable to set attribute {key:?} with value {value:?}: {err:?}")),
				None => element
					.remove_attribute(key)
					.unwrap_or_else(|err| panic!("Unable to remove attribute {key:?}: {err:?}")),
			});
		})
		.or_return()?;

		// Then set it
		self.attach_effect(attr_effect);
	}

	/// Adds a dynamic attribute to this element, with an empty value, given a predicate
	#[track_caller]
	fn set_dyn_attr_if<K, P>(&self, key: K, pred: P)
	where
		K: AsRef<str> + 'static,
		P: DynAttrPred + 'static,
	{
		self.set_dyn_attr(key, move || pred.eval().then_some(""));
	}
}

/// Extension trait to add reactive attribute to an element
#[extend::ext(name = ElementWithDynAttr)]
pub impl<E> E
where
	E: AsRef<web_sys::Element>,
{
	/// Adds a dynamic attribute to this element, where only the value is dynamic.
	///
	/// Returns the element, for chaining
	#[track_caller]
	fn with_dyn_attr<K, V>(self, key: K, value: V) -> Self
	where
		K: AsRef<str> + 'static,
		V: WithDynAttr + 'static,
	{
		self.as_ref().set_dyn_attr(key, value);
		self
	}

	/// Adds a dynamic attribute to this element, without a value, given a predicate
	///
	/// Returns the element, for chaining
	#[track_caller]
	fn with_dyn_attr_if<K, P>(self, key: K, pred: P) -> Self
	where
		K: AsRef<str> + 'static,
		P: DynAttrPred + 'static,
	{
		self.as_ref().set_dyn_attr_if(key, pred);
		self
	}
}

/// Trait for values accepted by [`ElementDynAttr::set_dyn_attr`].
///
/// This allows it to work with the following types:
/// - `N`
/// - `Signal<N>`
/// - `impl Fn() -> N`
///
/// Where `N` is a text type.
pub trait WithDynAttr {
	/// Calls `f` with the inner attribute
	fn with_attr<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&str>) -> O;
}

impl<FT, T> WithDynAttr for FT
where
	FT: Fn() -> T,
	T: WithDynAttr,
{
	fn with_attr<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&str>) -> O,
	{
		let text = self();
		text.with_attr(f)
	}
}

#[allow(clippy::allow_attributes, reason = "This only applies in some branches")]
#[allow(clippy::use_self, reason = "We always want to use `&str`, not `&Ty`")]
#[duplicate::duplicate_item(
	Ty;
	[str];
	[&'static str];
	[String];
)]
impl WithDynAttr for Ty {
	fn with_attr<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&str>) -> O,
	{
		f(Some(self))
	}
}

impl<T> WithDynAttr for Option<T>
where
	T: WithDynAttr,
{
	fn with_attr<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&str>) -> O,
	{
		match self {
			Some(s) => s.with_attr(f),
			None => f(None),
		}
	}
}

// TODO: Allow impl for `impl SignalGet<Value: WithDynText>`
#[duplicate::duplicate_item(
	Generics Ty;
	[T] [Signal<T> where T: WithDynAttr + 'static];
	[T, F] [Derived<T, F> where T: WithDynAttr + 'static, F: ?Sized + DerivedRun<T> + 'static];
	[T, F] [Memo<T, F> where T: WithDynAttr + 'static, F: ?Sized + 'static];
	[S, T] [WithDefault<S, T> where Self: for<'a> SignalWith<Value<'a>: Deref<Target: WithDynAttr>>];
)]
impl<Generics> WithDynAttr for Ty {
	fn with_attr<F2, O>(&self, f: F2) -> O
	where
		F2: FnOnce(Option<&str>) -> O,
	{
		self.with(|text| text.with_attr(f))
	}
}

/// Trait for values accepted by [`ElementDynAttr::set_dyn_attr_if`].
///
/// This allows it to work with the following types:
/// - `B`
/// - `Signal<B>`
/// - `impl Fn() -> B`
///
/// Where `B` is a boolean or type that implements `ToBoolDynAttr`
pub trait DynAttrPred {
	/// Evaluates this predicate
	fn eval(&self) -> bool;
}

impl<FT, T> DynAttrPred for FT
where
	FT: Fn() -> T,
	T: DynAttrPred,
{
	fn eval(&self) -> bool {
		self().eval()
	}
}

impl DynAttrPred for bool {
	fn eval(&self) -> bool {
		*self
	}
}

// TODO: Allow impl for `impl SignalGet<Value: WithDynText>`
#[duplicate::duplicate_item(
	Generics Ty;
	[T] [Signal<T> where T: DynAttrPred + 'static];
	[T, F] [Derived<T, F> where T: DynAttrPred + 'static, F: ?Sized + DerivedRun<T> + 'static];
	[T, F] [Memo<T, F> where T: DynAttrPred + 'static, F: ?Sized + 'static];
	[S, T] [WithDefault<S, T> where Self: for<'a> SignalWith<Value<'a>: Deref<Target: DynAttrPred>>];
)]
impl<Generics> DynAttrPred for Ty {
	fn eval(&self) -> bool {
		#[allow(
			clippy::allow_attributes,
			clippy::redundant_closure_for_method_calls,
			reason = "In some branches it isn't redundant"
		)]
		self.with(|value| value.eval())
	}
}
