//! Context

// Imports
use {
	super::{EnumSplitValue, SignalStorage},
	crate::{Effect, ReactiveWorld, Signal, SignalGetCloned, SignalSet, SignalWith},
	core::marker::PhantomData,
	zutil_cloned::cloned,
};

/// Context for [`EnumSplitValue::update`]
pub struct EnumSplitValueUpdateCtx<'a, S, W: ReactiveWorld> {
	/// Outer signal
	outer_signal: S,

	/// Effect currently running
	this_effect: Effect<W::F, W>,

	_phantom: PhantomData<&'a ()>,
}

impl<'a, S, W: ReactiveWorld> EnumSplitValueUpdateCtx<'a, S, W> {
	/// Creates a new context
	pub(crate) const fn new(outer_signal: S, this_effect: Effect<W::F, W>) -> Self {
		Self {
			outer_signal,
			this_effect,
			_phantom: PhantomData,
		}
	}

	/// Creates signal storage from a value
	pub fn create_signal_storage<T, V, F>(&self, value: V, into_t: F) -> SignalStorage<V>
	where
		T: EnumSplitValue<S, W>,
		S: SignalSet<T> + Clone + 'static,
		V: Clone + 'static,
		F: Fn(V) -> T + 'static,
	{
		let signal = Signal::new(value);

		// Create the write-back effect.
		// Note: We don't want to run it and write into the outer at startup, so
		//       we create it raw and add dependencies manually.
		#[cloned(inner_signal = signal, this_effect = self.this_effect, outer_signal = self.outer_signal)]
		let write_back_effect = Effect::new_raw(move || {
			let value = inner_signal.get_cloned();
			this_effect.suppressed(|| outer_signal.set(into_t(value)));
		});
		write_back_effect.gather_dependencies(|| {
			signal.with(|_| ());
		});

		SignalStorage::new(signal, write_back_effect)
	}
}
