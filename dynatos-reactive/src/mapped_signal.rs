//! Mapped signal

// Lints
#![expect(type_alias_bounds, reason = "We can't use `T::Residual` without the bound")]

// Imports
use {
	crate::{Effect, Signal, SignalGetCloned, SignalSet, SignalUpdate, SignalWith, Trigger, WeakEffect},
	core::{
		cell::{OnceCell, RefCell},
		ops::{ControlFlow, FromResidual, Residual, Try},
	},
	std::rc::Rc,
	zutil_cloned::cloned,
};

struct Inner<T>
where
	T: Try<Residual: Residual<Signal<T::Output>>>,
{
	/// Output signal
	output: OutputSignal<T>,

	// TODO: Make the effects not dynamic?
	/// Get effect
	_get_effect: Effect,

	/// Set effect
	_set_effect: Effect,

	/// Trigger
	trigger: Trigger,
}

/// Mapped signal.
///
/// Maps a signal, fallibly.
///
/// ```
/// # use dynatos_reactive::{Signal, SignalGetCloned, SignalGet, SignalSet, TryMappedSignal};
/// let outer = Signal::new(Some(5));
/// let mapped = TryMappedSignal::new(outer.clone(), |opt| *opt, |opt, &value| *opt = Some(value));
/// let inner = mapped.get_cloned().expect("Signal exists");
/// assert_eq!(inner.get(), 5);
///
/// // Writes into the inner signal change the outer signal
/// inner.set(6);
/// assert_eq!(outer.get(), Some(6));
///
/// // Writes into the outer signal change the inner signal,
/// // without re-running the current context...
/// outer.set(Some(6));
/// assert_eq!(inner.get(), 6);
///
/// // ... unless an error happens
/// outer.set(None);
/// assert!(mapped.get_cloned().is_none());
/// ```
// TODO: Just have the inner signal keep alive this?
///
/// # Lifetime
/// If you drop this signal, the relationship between
/// the outer and inner signal will be broken, so keep
/// this value alive while you use the inner signal
pub struct TryMappedSignal<T>
where
	T: Try<Residual: Residual<Signal<T::Output>>>,
{
	/// Inner
	inner: Rc<Inner<T>>,
}

impl<T> TryMappedSignal<T>
where
	T: Try<Residual: Residual<Signal<T::Output>>>,
{
	/// Creates a new mapped signal from a fallible getter
	pub fn new<S, TryGet, Set>(input: S, try_get: TryGet, set: Set) -> Self
	where
		T: 'static,
		S: SignalWith + SignalUpdate + Clone + 'static,
		TryGet: Fn(<S as SignalWith>::Value<'_>) -> T + 'static,
		Set: Fn(<S as SignalUpdate>::Value<'_>, &T::Output) + 'static,
	{
		// Output signal
		let output_sig = Rc::new(RefCell::new(None::<SignalTry<T>>));

		// Trigger for gathering dependencies on retrieving the output signal,
		// but *not* on output signal changes.
		let trigger = Trigger::new();

		// Weak reference to the `set_effect`, to ensure that we don't end
		// up with a loop and leak memory
		let set_weak_effect = Rc::new(OnceCell::<WeakEffect>::new());

		// The getter effect that sets the output signal
		#[cloned(input, output_sig, trigger, set_weak_effect)]
		let get_effect = Effect::new(move || {
			input.with(|input| {
				let value = try_get(input);

				let mut output = output_sig.borrow_mut();
				let (new_output, needs_trigger) = match value.branch() {
					// If the value was ok, check whether we already had a value or not
					ControlFlow::Continue(value) => match output.take().map(Try::branch) {
						// If we had a signal already, write to it
						Some(ControlFlow::Continue(signal)) => {
							// If we have the set effect, run it suppressed,
							// to avoid writing the value of the output signal
							// back into the input.
							let set_weak_effect = set_weak_effect.get().and_then(WeakEffect::upgrade);
							let _suppressed = set_weak_effect.as_ref().map(Effect::suppress);

							signal.set(value);

							(SignalTry::<T>::from_output(signal), false)
						},

						// Otherwise, we either had a failure, or nothing, so write a new signal
						// Note: If we're writing a new signal, we trigger if this isn't the first time running
						res => (SignalTry::<T>::from_output(Signal::new(value)), res.is_some()),
					},

					// If the value was an error, wipe the signal
					ControlFlow::Break(err) => (
						SignalTry::<T>::from_residual(err),
						output.take().map(Try::branch).is_some(),
					),
				};

				*output = Some(new_output);
				drop(output);
				if needs_trigger {
					trigger.exec();
				}
			});
		});


		// The set effect that writes the output back to the input
		let get_weak_effect = get_effect.downgrade();
		#[cloned(output_sig)]
		let set_effect = Effect::new_raw(move || {
			self::with_output_signal::<T, _>(&output_sig, |output| {
				// If we have the get effect, run it suppressed,
				// to avoid writing the value back into the output signal
				let get_weak_effect = get_weak_effect.upgrade();
				let _suppressed = get_weak_effect.as_ref().map(Effect::suppress);

				input.update(|input| output.with(|output| set(input, output)));
			});
		});
		set_effect.gather_deps(|| self::with_output_signal::<T, _>(&output_sig, |sig| sig.with(|_| ())));

		set_weak_effect
			.set(set_effect.downgrade())
			.expect("Set effect should be uninitialized");

		let inner = Rc::new(Inner {
			output: output_sig,
			_get_effect: get_effect,
			_set_effect: set_effect,
			trigger,
		});
		Self { inner }
	}
}

impl<T> Clone for TryMappedSignal<T>
where
	T: Try<Residual: Residual<Signal<T::Output>>>,
{
	fn clone(&self) -> Self {
		Self {
			inner: Rc::clone(&self.inner),
		}
	}
}

impl<T> SignalGetCloned for TryMappedSignal<T>
where
	T: Try<Residual: Residual<Signal<T::Output>>>,

	SignalTry<T>: Clone,
{
	type Value = SignalTry<T>;

	fn get_cloned(&self) -> Self::Value {
		self.inner.trigger.gather_subs();
		self.inner
			.output
			.borrow()
			.as_ref()
			.expect("Output signal was missing")
			.clone()
	}

	fn get_cloned_raw(&self) -> Self::Value {
		self.inner
			.output
			.borrow()
			.as_ref()
			.expect("Output signal was missing")
			.clone()
	}
}

/// Output signal type
type OutputSignal<T> = Rc<RefCell<Option<SignalTry<T>>>>;

/// Signal try type
type SignalTry<T: Try> = <T::Residual as Residual<Signal<T::Output>>>::TryType;


/// Accesses the inner type of an `OutputSignal`.
///
/// Assumes the inner value is populated
fn with_output_signal<T, F>(output_sig: &OutputSignal<T>, f: F)
where
	T: Try<Residual: Residual<Signal<T::Output>>>,
	F: FnOnce(&Signal<T::Output>),
{
	let mut output = output_sig.borrow_mut();

	// Take the existing type and branch on it
	let new_output = match output.take().expect("Output signal was missing").branch() {
		ControlFlow::Continue(sig) => {
			f(&sig);
			SignalTry::<T>::from_output(sig)
		},
		ControlFlow::Break(err) => SignalTry::<T>::from_residual(err),
	};

	*output = Some(new_output);
}

/// Mapped signal.
///
/// Maps a signal, infallibly.
pub struct MappedSignal<T>(TryMappedSignal<Result<T, !>>);

impl<T> MappedSignal<T> {
	/// Creates a new mapped signal from a fallible getter
	pub fn new<S, Get, Set>(input: S, get: Get, set: Set) -> Self
	where
		T: 'static,
		S: SignalWith + SignalUpdate + Clone + 'static,
		Get: Fn(<S as SignalWith>::Value<'_>) -> T + 'static,
		Set: Fn(<S as SignalUpdate>::Value<'_>, &T) + 'static,
	{
		Self(TryMappedSignal::new(
			input,
			move |value| Ok(get(value)),
			move |value, new_value| set(value, new_value),
		))
	}
}

impl<T> Clone for MappedSignal<T> {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

impl<T> SignalGetCloned for MappedSignal<T> {
	type Value = Signal<T>;

	fn get_cloned(&self) -> Self::Value {
		self.0.get_cloned().into_ok()
	}

	fn get_cloned_raw(&self) -> Self::Value {
		self.0.get_cloned_raw().into_ok()
	}
}

/// Extension trait to add a map a signal
#[extend::ext_sized(name = SignalMapped)]
pub impl<S> S
where
	S: SignalWith + SignalUpdate + Clone + 'static,
{
	/// Maps this signal fallibly
	fn try_mapped<T, TryGet, Set>(self, try_get: TryGet, set: Set) -> TryMappedSignal<T>
	where
		T: Try<Residual: Residual<Signal<T::Output>>> + 'static,
		TryGet: Fn(<S as SignalWith>::Value<'_>) -> T + 'static,
		Set: Fn(<S as SignalUpdate>::Value<'_>, &T::Output) + 'static,
	{
		TryMappedSignal::new(self, try_get, set)
	}

	/// Maps this signal
	fn mapped<T, Get, Set>(self, get: Get, set: Set) -> MappedSignal<T>
	where
		T: 'static,
		Get: Fn(<S as SignalWith>::Value<'_>) -> T + 'static,
		Set: Fn(<S as SignalUpdate>::Value<'_>, &T) + 'static,
	{
		MappedSignal::new(self, get, set)
	}
}


#[cfg(test)]
mod tests {
	use {
		super::*,
		crate::SignalGet,
		core::{assert_matches::assert_matches, cell::Cell},
	};

	#[test]
	fn basic() {
		let outer = Signal::new(Ok::<usize, ()>(5));

		// Counts the number of times that `outer` was written to
		#[thread_local]
		static TIMES_OUTER_CHANGED: Cell<usize> = Cell::new(0);
		#[cloned(outer)]
		let _effect = Effect::new(move || {
			_ = outer.get();
			TIMES_OUTER_CHANGED.set(TIMES_OUTER_CHANGED.get() + 1);
		});
		assert_eq!(TIMES_OUTER_CHANGED.get(), 1);

		let mapped = TryMappedSignal::new(outer.clone(), |opt| opt.ok(), |opt, &value| *opt = Ok(value));
		assert_eq!(TIMES_OUTER_CHANGED.get(), 1);

		{
			let inner = mapped.get_cloned().expect("Signal was missing");
			assert_eq!(TIMES_OUTER_CHANGED.get(), 1);
			assert_eq!(inner.get(), 5);

			outer.set(Ok(6));
			assert_eq!(TIMES_OUTER_CHANGED.get(), 2);
			assert_eq!(inner.get(), 6);

			inner.set(7);
			assert_eq!(TIMES_OUTER_CHANGED.get(), 3);
			assert_eq!(outer.get(), Ok(7));
		};

		{
			outer.set(Err(()));
			assert_matches!(mapped.get_cloned(), None);
		};

		{
			outer.set(Ok(1));
			let inner = mapped.get_cloned().expect("Signal was missing");
			assert_eq!(inner.get(), 1);
		}
	}

	#[test]
	fn effects() {
		let outer = Signal::new(Ok::<usize, usize>(5));
		let mapped = TryMappedSignal::new(outer.clone(), |opt| *opt, |opt, &value| *opt = Ok(value));

		// Counts the times that the mapped signal was run
		#[thread_local]
		static TIMES_RUN: Cell<usize> = Cell::new(0);
		let _effect = Effect::new(move || {
			_ = mapped.get_cloned();
			TIMES_RUN.set(TIMES_RUN.get() + 1);
		});

		assert_eq!(TIMES_RUN.get(), 1);
		outer.set(Ok(6));
		assert_eq!(TIMES_RUN.get(), 1);
		outer.set(Err(1));
		assert_eq!(TIMES_RUN.get(), 2);
		outer.set(Err(2));
		assert_eq!(TIMES_RUN.get(), 3);
		outer.set(Ok(1));
		assert_eq!(TIMES_RUN.get(), 4);
		outer.set(Ok(2));
		assert_eq!(TIMES_RUN.get(), 4);
	}
}
