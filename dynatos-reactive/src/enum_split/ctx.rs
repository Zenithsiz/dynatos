//! Context

// Imports
use {
	super::{EnumSplitValue, SignalStorage},
	crate::{effect, Effect, Signal, SignalGetCloned, SignalSet, SignalWith},
	core::marker::PhantomData,
	zutil_cloned::cloned,
};

/// Context for [`EnumSplitValue::update`]
pub struct EnumSplitValueUpdateCtx<'a, S> {
	/// Outer signal
	outer_signal: S,

	_phantom: PhantomData<&'a ()>,
}

impl<S> EnumSplitValueUpdateCtx<'_, S> {
	/// Creates a new context.
	pub(crate) const fn new(outer_signal: S) -> Self {
		Self {
			outer_signal,
			_phantom: PhantomData,
		}
	}

	/// Creates signal storage from a value.
	///
	/// Must be run from inside of an effect.
	/// Suppresses the current effect during
	/// the write-back effect of the signal storage
	/// created, to avoid recursion
	pub fn create_signal_storage<T, V, F>(&self, value: V, into_t: F) -> SignalStorage<V>
	where
		T: EnumSplitValue<S>,
		S: SignalSet<T> + Clone + 'static,
		V: Clone + 'static,
		F: Fn(V) -> T + 'static,
	{
		let signal = Signal::new(value);

		let cur_effect = effect::running().expect("Missing running effect");

		// Create the write-back effect.
		// Note: We don't want to run it and write into the outer at startup, so
		//       we create it raw and add dependencies manually.
		#[cloned(inner_signal = signal, outer_signal = self.outer_signal)]
		let write_back_effect = Effect::new_raw(move || {
			let value = inner_signal.get_cloned();
			let _suppressed = cur_effect.suppress();
			outer_signal.set(into_t(value));
		});
		write_back_effect.gather_dependencies(|| {
			signal.with(|_| ());
		});

		SignalStorage::new(signal, write_back_effect)
	}
}
