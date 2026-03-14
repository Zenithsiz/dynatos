//! Dynamic element

use {
	crate::ObjectAttachEffect,
	core::{
		cell::{LazyCell, RefCell},
		ops::Deref,
	},
	dynatos_html::{Child, html},
	dynatos_reactive::{Derived, Effect, Memo, Signal, SignalWith, WithDefault, derived::DerivedRun, effect},
	dynatos_util::TryOrReturnExt,
	js_sys::WeakRef,
	std::{rc::Rc, sync::LazyLock},
};

/// A dynamic element
pub struct DynElement(Rc<RefCell<web_sys::Element>>);

impl DynElement {
	/// Creates a new dynamic element
	pub fn new<T>(f: T) -> Self
	where
		T: ToDynElement + 'static,
	{
		let default_element = web_sys::Element::from(html::template());
		let element_weak_ref = RefCell::new(WeakRef::<web_sys::Element>::new(&default_element));

		let element = Rc::new(RefCell::new(default_element));
		let element_weak_rc = Rc::downgrade(&element);

		let _ = Effect::new(move || {
			// If our element is gone, we can safely quit.
			let cur_element = WeakRef::deref(&element_weak_ref.borrow()).or_return()?;

			// When creating a new effect, always re-attach the effect to it.
			let this_effect = effect::running().expect("Should have an effect running");
			let new_element = f.to_element();
			new_element.attach_effect(this_effect);

			// Replace the element in both the dom, our weak ref to it, and
			// the reference in the dyn element, if it's still alive.
			cur_element
				.replace_with_with_node_1(&new_element)
				.expect("Unable to replace element");
			*element_weak_ref.borrow_mut() = WeakRef::new(&new_element);
			if let Some(element) = element_weak_rc.upgrade() {
				*element.borrow_mut() = new_element;
			}
		});

		Self(element)
	}
}

impl Child for DynElement {
	fn append(&self, node: &web_sys::Node) -> Result<(), wasm_bindgen::JsValue> {
		self.0.borrow().append(node)
	}
}

/// Trait for values accepted by [`DynElement`].
///
/// This allows it to work with the following types:
/// - `impl Fn() -> N`
/// - `web_sys::{Element, Element, HtmlElement}`
/// - `Option<N>`
/// - [`Signal`], [`Derived`], [`Memo`], [`WithDefault`]
/// - `LazyCell<N, impl Fn() -> N>`
/// - `!`
///
/// Where `N` is any of the types above.
pub trait ToDynElement {
	/// Gets the element
	fn to_element(&self) -> web_sys::Element;
}

impl<F, N> ToDynElement for F
where
	F: Fn() -> N,
	N: ToDynElement,
{
	fn to_element(&self) -> web_sys::Element {
		self().to_element()
	}
}

// TODO: Impl for `impl AsRef<web_sys::Element>` if we can get rid of
//       the conflict with the function impl
#[allow(clippy::allow_attributes, reason = "This only applies in some branches")]
#[allow(clippy::use_self, reason = "We always want to use `web_sys::Element`, not `Ty`")]
#[duplicate::duplicate_item(
	Ty;
	[web_sys::Element];
	[web_sys::HtmlElement];
)]
impl ToDynElement for Ty {
	fn to_element(&self) -> web_sys::Element {
		<Self as AsRef<web_sys::Element>>::as_ref(self).clone()
	}
}

// TODO: Allow impl for `impl SignalWith<Value: ToDynElement>`
#[duplicate::duplicate_item(
	Generics Ty;
	[T] [Signal<T> where T: ToDynElement + 'static];
	[T, F] [Derived<T, F> where T: ToDynElement + 'static, F: ?Sized + DerivedRun<T> + 'static];
	[T, F] [Memo<T, F> where T: ToDynElement + 'static, F: ?Sized + 'static];
	[S, T] [WithDefault<S, T> where Self: for<'a> SignalWith<Value<'a>: Deref<Target: ToDynElement>>];
)]
impl<Generics> ToDynElement for Ty {
	fn to_element(&self) -> web_sys::Element {
		#[allow(
			clippy::allow_attributes,
			clippy::redundant_closure_for_method_calls,
			reason = "In some branches it isn't redundant"
		)]
		self.with(|value| value.to_element())
	}
}

impl<N, F> ToDynElement for LazyCell<N, F>
where
	N: ToDynElement,
	F: FnOnce() -> N,
{
	fn to_element(&self) -> web_sys::Element {
		(**self).to_element()
	}
}

impl<N, F> ToDynElement for LazyLock<N, F>
where
	N: ToDynElement,
	F: FnOnce() -> N,
{
	fn to_element(&self) -> web_sys::Element {
		(**self).to_element()
	}
}

impl ToDynElement for ! {
	fn to_element(&self) -> web_sys::Element {
		*self
	}
}
