//! Dynamic element

// Imports
use {
	crate::ObjectAttachEffect,
	core::ops::Deref,
	dynatos_reactive::{Derived, Effect, Memo, Signal, SignalWith, WithDefault, derived::DerivedRun, effect},
	dynatos_sync_types::{IMut, RcPtr, SyncBounds},
	dynatos_util::TryOrReturnExt,
	dynatos_web::{Child, DynatosWebCtx, html},
	dynatos_web::types::{Element, HtmlElement, Node, WeakRef, WebError},
};

/// A dynamic element
pub struct DynElement(RcPtr<IMut<Element>>);

impl DynElement {
	/// Creates a new dynamic element
	pub fn new<T>(ctx: &DynatosWebCtx, f: T) -> Self
	where
		T: ToDynElement + 'static,
	{
		let default_element = Element::from(html::template(ctx));
		let element_weak_ref = IMut::new(WeakRef::<Element>::new(&default_element));

		let element = RcPtr::new(IMut::new(default_element));
		let element_weak_rc = RcPtr::downgrade(&element);

		let _ = Effect::new(move || {
			// If our element is gone, we can safely quit.
			let cur_element = WeakRef::deref(&element_weak_ref.lock()).or_return()?;

			// When creating a new effect, always re-attach the effect to it.
			let this_effect = effect::running().expect("Should have an effect running");
			let new_element = f.to_element();
			new_element.attach_effect(this_effect);

			// Replace the element in both the dom, our weak ref to it, and
			// the reference in the dyn element, if it's still alive.
			cur_element
				.replace_with_with_node_1(&new_element)
				.expect("Unable to replace element");

			*element_weak_ref.lock() = WeakRef::new(&new_element);
			if let Some(element) = element_weak_rc.upgrade() {
				*element.lock() = new_element;
			}
		});

		Self(element)
	}
}

impl Child for DynElement {
	fn append(&self, node: &Node) -> Result<(), WebError> {
		self.0.lock().append(node)
	}
}

/// Trait for values accepted by [`DynElement`].
///
/// This allows it to work with the following types:
/// - `impl Fn() -> N`
/// - `{Element, Element, HtmlElement}`
/// - `Option<N>`
/// - [`Signal`], [`Derived`], [`Memo`], [`WithDefault`]
/// - `LazyCell<N, impl Fn() -> N>`
/// - `!`
///
/// Where `N` is any of the types above.
pub trait ToDynElement: SyncBounds {
	/// Gets the element
	fn to_element(&self) -> Element;
}

impl<F, N> ToDynElement for F
where
	F: SyncBounds + Fn() -> N,
	N: ToDynElement,
{
	fn to_element(&self) -> Element {
		self().to_element()
	}
}

// TODO: Impl for `impl AsRef<Element>` if we can get rid of
//       the conflict with the function impl
#[allow(clippy::allow_attributes, reason = "This only applies in some branches")]
#[allow(clippy::use_self, reason = "We always want to use `Element`, not `Ty`")]
#[duplicate::duplicate_item(
	Ty;
	[Element];
	[HtmlElement];
)]
impl ToDynElement for Ty {
	fn to_element(&self) -> Element {
		<Self as AsRef<Element>>::as_ref(self).clone()
	}
}

// TODO: Allow impl for `impl SignalWith<Value: ToDynElement>`
#[duplicate::duplicate_item(
	Generics Ty;
	[T] [Signal<T> where T: ToDynElement + 'static];
	[T, F] [Derived<T, F> where T: ToDynElement + 'static, F: ?Sized + DerivedRun<T> + 'static];
	[T, F] [Memo<T, F> where T: ToDynElement + 'static, F: SyncBounds + ?Sized + 'static];
	[S, T] [WithDefault<S, T> where S: SyncBounds, T: SyncBounds, Self: for<'a> SignalWith<Value<'a>: Deref<Target: ToDynElement>>];
)]
impl<Generics> ToDynElement for Ty {
	fn to_element(&self) -> Element {
		#[allow(
			clippy::allow_attributes,
			clippy::redundant_closure_for_method_calls,
			reason = "In some branches it isn't redundant"
		)]
		self.with(|value| value.to_element())
	}
}

#[expect(clippy::absolute_paths, reason = "We want to be explicit due to the `sync` feature")]
impl<N, F> ToDynElement for core::cell::LazyCell<N, F>
where
	N: ToDynElement,
	F: FnOnce() -> N,
	Self: SyncBounds,
{
	fn to_element(&self) -> Element {
		(**self).to_element()
	}
}

#[expect(clippy::absolute_paths, reason = "We want to be explicit due to the `sync` feature")]
impl<N, F> ToDynElement for std::sync::LazyLock<N, F>
where
	N: ToDynElement,
	F: FnOnce() -> N,
	Self: SyncBounds,
{
	fn to_element(&self) -> Element {
		(**self).to_element()
	}
}

impl ToDynElement for ! {
	fn to_element(&self) -> Element {
		*self
	}
}
