//! Effect run

// Imports
use {super::Inner, core::marker::PhantomData, std::rc::Rc};

/// Effect run
///
/// # Implementation
/// To implement this trait, you must implement the [`run`](EffectRun::run) function,
/// and then use the macro [`effect_run_impl_inner`] to implement some details.
pub trait EffectRun {
	/// Runs the effect
	#[track_caller]
	fn run(&self, ctx: EffectRunCtx<'_>);

	// Implementation details.

	/// Unsizes the inner field of the effect
	#[doc(hidden)]
	fn unsize_inner(self: Rc<Inner<Self>>) -> Rc<Inner<dyn EffectRun>>;
}

impl EffectRun for ! {
	effect_run_impl_inner! {}

	#[coverage(off)]
	fn run(&self, _ctx: EffectRunCtx<'_>) {
		*self
	}
}

impl<F> EffectRun for F
where
	F: Fn() + 'static,
{
	effect_run_impl_inner! {}

	fn run(&self, _ctx: EffectRunCtx<'_>) {
		self();
	}
}

/// Implementation detail for the [`EffectRun`] trait
pub macro effect_run_impl_inner() {
	fn unsize_inner(self: Rc<Inner<Self>>) -> Rc<Inner<dyn EffectRun>> {
		self
	}
}

/// Effect run context
pub struct EffectRunCtx<'a> {
	_phantom: PhantomData<&'a ()>,
}

impl EffectRunCtx<'_> {
	/// Creates new context for running an effect
	pub(crate) const fn new() -> Self {
		Self { _phantom: PhantomData }
	}
}
