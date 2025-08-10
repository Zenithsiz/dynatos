//! Html element reactive css property

// Imports
use {
	crate::ObjectAttachEffect,
	core::ops::Deref,
	dynatos_html::WeakRef,
	dynatos_reactive::{Derived, Effect, Memo, Signal, SignalWith, WithDefault, derived::DerivedRun},
	dynatos_util::TryOrReturnExt,
};

/// Extension trait to add reactive css properties to an html element
#[extend::ext(name = HtmlElementDynCssProp)]
pub impl web_sys::HtmlElement {
	/// Adds a dynamic css property to this element
	#[track_caller]
	fn set_dyn_css_prop<K, V>(&self, key: K, value: V)
	where
		K: AsRef<str> + 'static,
		V: WithDynCssProp + 'static,
	{
		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the element.
		//       Otherwise, the element will be keeping us alive, while we keep
		//       the element alive, causing a leak.
		let element = WeakRef::new(self);
		let prop_effect = Effect::try_new(move || {
			// Try to get the element
			let element = element.get().or_return()?;

			// And set the property
			let key = key.as_ref();
			value.with_prop(|value| match value {
				Some(value) => element
					.style()
					.set_property(key, value)
					.unwrap_or_else(|err| panic!("Unable to set css property {key:?} with value {value:?}: {err:?}")),
				None =>
					_ = element
						.style()
						.remove_property(key)
						.unwrap_or_else(|err| panic!("Unable to remove css property {key:?}: {err:?}")),
			});
		})
		.or_return()?;

		// Then set it
		self.attach_effect(prop_effect);
	}

	/// Adds a dynamic css property to this element, with an empty value, given a predicate
	#[track_caller]
	fn set_dyn_css_prop_if<K, P>(&self, key: K, pred: P)
	where
		K: AsRef<str> + 'static,
		P: DynCssPropPred + 'static,
	{
		self.set_dyn_css_prop(key, move || pred.eval().then_some(""));
	}
}

/// Extension trait to add reactive css property to an element
#[extend::ext(name = HtmlElementWithDynCssProp)]
pub impl<E> E
where
	E: AsRef<web_sys::HtmlElement>,
{
	/// Adds a dynamic css property to this element, where only the value is dynamic.
	///
	/// Returns the element, for chaining
	#[track_caller]
	fn with_dyn_css_prop<K, V>(self, key: K, value: V) -> Self
	where
		K: AsRef<str> + 'static,
		V: WithDynCssProp + 'static,
	{
		self.as_ref().set_dyn_css_prop(key, value);
		self
	}

	/// Adds a dynamic css property to this element, without a value, given a predicate
	///
	/// Returns the element, for chaining
	#[track_caller]
	fn with_dyn_css_prop_if<K, P>(self, key: K, pred: P) -> Self
	where
		K: AsRef<str> + 'static,
		P: DynCssPropPred + 'static,
	{
		self.as_ref().set_dyn_css_prop_if(key, pred);
		self
	}
}

/// Trait for values accepted by [`HtmlElementDynCssProp::set_dyn_css_prop`].
///
/// This allows it to work with the following types:
/// - `N`
/// - `Signal<N>`
/// - `impl Fn() -> N`
///
/// Where `N` is a text type.
pub trait WithDynCssProp {
	/// Calls `f` with the inner css property
	fn with_prop<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&str>) -> O;
}

impl<FT, T> WithDynCssProp for FT
where
	FT: Fn() -> T,
	T: WithDynCssProp,
{
	fn with_prop<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&str>) -> O,
	{
		let text = self();
		text.with_prop(f)
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
impl WithDynCssProp for Ty {
	fn with_prop<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&str>) -> O,
	{
		f(Some(self))
	}
}

impl<T> WithDynCssProp for Option<T>
where
	T: WithDynCssProp,
{
	fn with_prop<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&str>) -> O,
	{
		match self {
			Some(s) => s.with_prop(f),
			None => f(None),
		}
	}
}

// TODO: Allow impl for `impl SignalGet<Value: WithDynText>`
#[duplicate::duplicate_item(
	Generics Ty;
	[T] [Signal<T> where T: WithDynCssProp + 'static];
	[T, F] [Derived<T, F> where T: WithDynCssProp + 'static, F: ?Sized + DerivedRun<T> + 'static];
	[T, F] [Memo<T, F> where T: WithDynCssProp + 'static, F: ?Sized + 'static];
	[S, T] [WithDefault<S, T> where Self: for<'a> SignalWith<Value<'a>: Deref<Target: WithDynCssProp>>];
)]
impl<Generics> WithDynCssProp for Ty {
	fn with_prop<F2, O>(&self, f: F2) -> O
	where
		F2: FnOnce(Option<&str>) -> O,
	{
		self.with(|text| text.with_prop(f))
	}
}

/// Trait for values accepted by [`HtmlElementDynCssProp::set_dyn_css_prop_if`].
///
/// This allows it to work with the following types:
/// - `B`
/// - `Signal<B>`
/// - `impl Fn() -> B`
///
/// Where `B` is a boolean or type that implements [`DynCssPropPred`]
pub trait DynCssPropPred {
	/// Evaluates this predicate
	fn eval(&self) -> bool;
}

impl<FT, T> DynCssPropPred for FT
where
	FT: Fn() -> T,
	T: DynCssPropPred,
{
	fn eval(&self) -> bool {
		self().eval()
	}
}

impl DynCssPropPred for bool {
	fn eval(&self) -> bool {
		*self
	}
}

// TODO: Allow impl for `impl SignalGet<Value: WithDynText>`
#[duplicate::duplicate_item(
	Generics Ty;
	[T] [Signal<T> where T: DynCssPropPred + 'static];
	[T, F] [Derived<T, F> where T: DynCssPropPred + 'static, F: ?Sized + DerivedRun<T> + 'static];
	[T, F] [Memo<T, F> where T: DynCssPropPred + 'static, F: ?Sized + 'static];
	[S, T] [WithDefault<S, T> where Self: for<'a> SignalWith<Value<'a>: Deref<Target: DynCssPropPred>>];
)]
impl<Generics> DynCssPropPred for Ty {
	fn eval(&self) -> bool {
		#[allow(
			clippy::allow_attributes,
			clippy::redundant_closure_for_method_calls,
			reason = "In some branches it isn't redundant"
		)]
		self.with(|value| value.eval())
	}
}
