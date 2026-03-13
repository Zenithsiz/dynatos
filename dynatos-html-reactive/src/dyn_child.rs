//! Dynamic child

use {
	crate::ObjectAttachEffect,
	core::{
		cell::{LazyCell, RefCell},
		ops::Deref,
	},
	dynatos_reactive::{Derived, Effect, Memo, Signal, SignalWith, WithDefault, derived::DerivedRun, effect},
	dynatos_util::TryOrReturnExt,
	js_sys::WeakRef,
	std::sync::{LazyLock, oneshot},
};

/// A dynamic child
pub struct DynChild(web_sys::Element);

impl DynChild {
	/// Creates a new dynamic child
	pub fn new<T>(f: T) -> Self
	where
		T: ToDynElement + 'static,
	{
		struct State {
			// TODO: Not need to send the element via a channel here.
			send_el: Option<oneshot::Sender<web_sys::Element>>,
			element: Option<WeakRef<web_sys::Element>>,
		}

		let (send_el, recv_el) = oneshot::channel();
		let state = RefCell::new(State {
			send_el: Some(send_el),
			element: None,
		});

		Effect::new(move || {
			let cur_element = match &state.borrow().element {
				// Note: If we already had an element, but it's been dropped, then
				//       there's nothing to do, and we'll get garbage collected soon
				Some(element) => Some(element.deref().or_return()?),
				// This only happens on the first invocation.
				None => None,
			};

			// When creating a new effect, always re-attach the effect to it.
			let this_effect = effect::running().expect("Should have an effect running");
			let new_element = f.to_element();
			new_element.attach_effect(this_effect);

			// Then if we had an existing element, replace it with the new one
			if let Some(cur_element) = cur_element {
				cur_element
					.replace_with_with_node_1(&new_element)
					.expect("Unable to replace element");
			}

			// And finally update the state.
			let mut state = state.borrow_mut();
			state.element = Some(WeakRef::new(&new_element));

			// If this is the first invocation, send the element through
			// the channel.
			if let Some(send_el) = state.send_el.take() {
				_ = send_el.send(new_element);
			}
		});
		let element = recv_el.recv().expect("Should have an element");

		Self(element)
	}
}

#[duplicate::duplicate_item(
	Ty;
	[web_sys::Node];
	[web_sys::Element];
)]
impl AsRef<Ty> for DynChild {
	fn as_ref(&self) -> &Ty {
		&self.0
	}
}

/// Trait for values accepted by [`ElementDynChildren`].
///
/// This allows it to work with the following types:
/// - `impl Fn() -> N`
/// - `web_sys::{Element, Element, HtmlElement}`
/// - `Option<N>`
/// - `Vec<N>`, `[N; _]`, `[N]`
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
