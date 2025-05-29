//! Enum split signal

// Modules
mod either;

// Exports
pub use self::either::{All1, All2, All3, Either1, Either2, Either3};

// Imports
use {
	crate::{
		Effect,
		EffectRun,
		EffectRunCtx,
		ReactiveWorld,
		Signal,
		SignalBorrow,
		SignalGetCloned,
		SignalSet,
		SignalWith,
		SignalWithDefaultImpl,
		Trigger,
	},
	core::{
		fmt,
		marker::{PhantomData, Unsize},
	},
	dynatos_world::{IMut, IMutLike, WorldDefault},
	zutil_cloned::cloned,
};

/// World for [`EnumSplitSignal`]
#[expect(private_bounds, reason = "We can't *not* leak some implementation details currently")]
pub trait EnumSplitWorld<S, T: EnumSplitValue<S, Self>> =
	ReactiveWorld where IMut<EffectFnInner<S, T, Self>, Self>: Sized;

/// Enum split signal
pub struct EnumSplitSignal<S, T: EnumSplitValue<S, W>, W: EnumSplitWorld<S, T> = WorldDefault> {
	/// Effect
	effect: Effect<EffectFn<S, T, W>, W>,
}

impl<S, T: EnumSplitValue<S, WorldDefault>> EnumSplitSignal<S, T, WorldDefault> {
	/// Creates a new enum split signal
	pub fn new(signal: S) -> Self
	where
		T: 'static,
		S: SignalGetCloned<Value = T> + SignalSet<T> + Clone + 'static,
	{
		Self::new_in(signal, WorldDefault::default())
	}
}

impl<S, T: EnumSplitValue<S, W>, W: EnumSplitWorld<S, T>> EnumSplitSignal<S, T, W> {
	/// Creates a new enum split signal in a world
	#[expect(private_bounds, reason = "We can't *not* leak some implementation details currently")]
	pub fn new_in(signal: S, world: W) -> Self
	where
		T: 'static,
		S: SignalGetCloned<Value = T> + SignalSet<T> + Clone + 'static,
		EffectFn<S, T, W>: Unsize<W::F>,
	{
		let effect = Effect::new_in(
			EffectFn {
				inner: IMut::<_, W>::new(EffectFnInner::default()),
				trigger: Trigger::new(),
				signal,
			},
			world,
		);

		Self { effect }
	}
}

impl<S, T: EnumSplitValue<S, W>, W: EnumSplitWorld<S, T>> Clone for EnumSplitSignal<S, T, W> {
	fn clone(&self) -> Self {
		Self {
			effect: self.effect.clone(),
		}
	}
}

impl<S, T, W> fmt::Debug for EnumSplitSignal<S, T, W>
where
	T: EnumSplitValue<S, W>,
	T::SignalsStorage: fmt::Debug,
	T::SigKind: fmt::Debug,
	W: EnumSplitWorld<S, T>,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut s = f.debug_struct("EnumSplitSignal");
		s.field("effect", &self.effect);

		match self.effect.inner_fn().inner.try_read() {
			Some(inner) => s.field("signals", &inner.signals).field("cur", &inner.cur).finish(),
			None => s.finish_non_exhaustive(),
		}
	}
}

impl<S, T: EnumSplitValue<S, W>, W: EnumSplitWorld<S, T>> SignalBorrow for EnumSplitSignal<S, T, W> {
	type Ref<'a>
		= T::Signal
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		self.effect.inner_fn().trigger.gather_subscribers();
		self.borrow_raw()
	}

	fn borrow_raw(&self) -> Self::Ref<'_> {
		let effect_fn = self.effect.inner_fn();

		let inner = effect_fn.inner.read();

		let cur = inner.cur.as_ref().expect("Should have a current signal");
		T::get_signal(&inner.signals, cur).expect("Signal for current signal was missing")
	}
}

impl<S, T: EnumSplitValue<S, W>, W: EnumSplitWorld<S, T>> SignalWithDefaultImpl for EnumSplitSignal<S, T, W> {}

/// Signal storage.
#[derive(Debug)]
pub struct SignalStorage<T> {
	/// Signal
	// TODO: Allow the user to specify another type of signal?
	signal: Signal<T>,

	/// Write-back effect
	// TODO: Not use dynamic dispatch here
	write_back_effect: Effect<dyn EffectRun>,
}

impl<T> SignalStorage<T> {
	/// Clones the signal in storage
	#[must_use]
	pub fn signal(&self) -> Signal<T> {
		self.signal.clone()
	}

	/// Sets the value of the signal in storage.
	///
	/// Suppresses the write-back effect during.
	pub fn set(&self, new_value: T)
	where
		T: 'static,
	{
		self.write_back_effect.suppressed(|| self.signal.set(new_value));
	}
}

/// Effect fn inner
struct EffectFnInner<S, T: EnumSplitValue<S, W>, W: ReactiveWorld> {
	/// Signals
	signals: T::SignalsStorage,

	/// Current signal
	cur: Option<T::SigKind>,
}

impl<S, T: EnumSplitValue<S, W>, W: ReactiveWorld> Default for EffectFnInner<S, T, W> {
	fn default() -> Self {
		Self {
			signals: T::SignalsStorage::default(),
			cur:     None,
		}
	}
}

/// Inner effect function
struct EffectFn<S, T: EnumSplitValue<S, W>, W: EnumSplitWorld<S, T>> {
	/// Inner
	inner: IMut<EffectFnInner<S, T, W>, W>,

	/// Trigger
	trigger: Trigger,

	/// Signal
	signal: S,
}

impl<S, T, W> EffectRun<W> for EffectFn<S, T, W>
where
	T: EnumSplitValue<S, W>,
	S: SignalGetCloned<Value = T> + SignalSet<T> + Clone + 'static,
	W: EnumSplitWorld<S, T>,
{
	fn run(&self, ctx: EffectRunCtx<'_, W>) {
		// Get the new value
		let new_value = self.signal.get_cloned();

		// Then update the current signal
		let mut inner = self.inner.write();
		let prev_cur = inner.cur.replace(new_value.kind());
		let ctx = EnumSplitValueUpdateCtx {
			outer_signal: self.signal.clone(),
			this_effect:  ctx.effect(),
			_phantom:     PhantomData,
		};
		new_value.update(&mut inner.signals, ctx);

		if prev_cur != inner.cur {
			drop(inner);
			self.trigger.exec();
		}
	}
}

/// Enum split value
pub trait EnumSplitValue<S, W: ReactiveWorld = WorldDefault> {
	/// Signals storage
	type SignalsStorage: Default;

	/// Signal type
	type Signal;

	/// Signal kind
	type SigKind: PartialEq;

	/// Extracts a signal from storage
	fn get_signal(storage: &Self::SignalsStorage, kind: &Self::SigKind) -> Option<Self::Signal>;

	/// Gets the signal kind of this value
	fn kind(&self) -> Self::SigKind;

	/// Updates a signal with this value
	fn update(self, storage: &mut Self::SignalsStorage, ctx: EnumSplitValueUpdateCtx<'_, S, W>);
}

/// Context for [`EnumSplitValue::update`]
pub struct EnumSplitValueUpdateCtx<'a, S, W: ReactiveWorld> {
	/// Outer signal
	outer_signal: S,

	/// Effect currently running
	this_effect: Effect<W::F, W>,

	_phantom: PhantomData<&'a ()>,
}

impl<'a, S, W: ReactiveWorld> EnumSplitValueUpdateCtx<'a, S, W> {
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

		SignalStorage {
			signal,
			write_back_effect,
		}
	}
}

impl<S, T, W> EnumSplitValue<S, W> for Option<T>
where
	T: Clone + 'static,
	S: SignalSet<Self> + Clone + 'static,
	W: ReactiveWorld,
{
	type SigKind = Option<()>;
	type Signal = Option<Signal<T>>;
	type SignalsStorage = Option<SignalStorage<T>>;

	fn get_signal(storage: &Self::SignalsStorage, cur: &Self::SigKind) -> Option<Self::Signal> {
		let signal = match cur {
			Some(()) => Some(storage.as_ref()?.signal()),
			None => None,
		};

		Some(signal)
	}

	fn kind(&self) -> Self::SigKind {
		self.as_ref().map(|_| ())
	}

	fn update(self, storage: &mut Self::SignalsStorage, ctx: EnumSplitValueUpdateCtx<'_, S, W>) {
		let Some(new_value) = self else { return };

		match storage {
			Some(storage) => storage.set(new_value),
			None => *storage = Some(ctx.create_signal_storage(new_value, Some)),
		}
	}
}

/// Extension trait to create an enum split signal
// TODO: Add this for other worlds
#[extend::ext_sized(name = SignalEnumSplit)]
pub impl<S> S {
	/// Splits this signal into sub-signals
	fn enum_split<T>(self) -> EnumSplitSignal<S, T>
	where
		T: EnumSplitValue<S> + 'static,
		S: SignalGetCloned<Value = T> + SignalSet<T> + Clone + 'static,
	{
		EnumSplitSignal::new(self)
	}
}


#[cfg(test)]
mod tests {
	use {
		super::*,
		crate::{Effect, Signal, SignalGet},
		core::cell::{Cell, OnceCell},
		zutil_cloned::cloned,
	};

	#[test]
	fn exec() {
		let input = Signal::new(Either2::<i32, ()>::T2(()));

		#[cloned(input)]
		let signal = EnumSplitSignal::new(input);

		#[thread_local]
		static EFFECT_SOME: OnceCell<Effect<dyn Fn()>> = OnceCell::new();

		#[thread_local]
		static EFFECT_NONE: OnceCell<Effect<dyn Fn()>> = OnceCell::new();

		#[thread_local]
		static TIMES_CHANGED_SOME: Cell<usize> = Cell::new(0);

		#[thread_local]
		static TIMES_CHANGED_NONE: Cell<usize> = Cell::new(0);

		#[thread_local]
		static TIMES_RUN_SOME: Cell<usize> = Cell::new(0);

		#[thread_local]
		static TIMES_RUN_NONE: Cell<usize> = Cell::new(0);

		#[cloned(signal)]
		let _effect = Effect::new(move || match signal.borrow() {
			Either2::T1(signal) => {
				TIMES_CHANGED_SOME.set(TIMES_CHANGED_SOME.get() + 1);
				EFFECT_SOME.get_or_init(|| {
					Effect::new(move || {
						_ = signal.get();
						TIMES_RUN_SOME.set(TIMES_RUN_SOME.get() + 1);
					})
				});
			},
			Either2::T2(signal) => {
				TIMES_CHANGED_NONE.set(TIMES_CHANGED_NONE.get() + 1);
				EFFECT_NONE.get_or_init(|| {
					Effect::new(move || {
						() = signal.get();
						TIMES_RUN_NONE.set(TIMES_RUN_NONE.get() + 1);
					})
				});
			},
		});

		fn get_times() -> [usize; 4] {
			[
				TIMES_CHANGED_SOME.get(),
				TIMES_RUN_SOME.get(),
				TIMES_CHANGED_NONE.get(),
				TIMES_RUN_NONE.get(),
			]
		}

		assert_eq!(get_times(), [0, 0, 1, 1]);

		input.set(Either2::T2(()));
		assert_eq!(get_times(), [0, 0, 1, 2]);

		input.set(Either2::T1(1));
		assert_eq!(get_times(), [1, 1, 1, 2]);

		input.set(Either2::T1(2));
		assert_eq!(get_times(), [1, 2, 1, 2]);

		input.set(Either2::T2(()));
		assert_eq!(get_times(), [1, 2, 2, 3]);

		input.set(Either2::T1(3));
		assert_eq!(get_times(), [2, 3, 2, 3]);
	}

	#[test]
	fn write_back() {
		// Start with `T1`
		let outer = Signal::new(Either2::<i32, &'static str>::T1(5));
		#[thread_local]
		static TIMES_RUN_OUTER: Cell<usize> = Cell::new(0);
		#[cloned(outer)]
		let _effect_outer = Effect::new(move || {
			_ = outer.get();
			TIMES_RUN_OUTER.set(TIMES_RUN_OUTER.get() + 1);
		});

		// Get the `T1` signal
		let signal = EnumSplitSignal::new(outer.clone());
		let Either2::T1(inner1) = signal.borrow() else {
			unreachable!("Signal was the wrong type at the beginning")
		};
		assert_eq!(TIMES_RUN_OUTER.get(), 1);
		#[thread_local]
		static TIMES_RUN_INNER1: Cell<usize> = Cell::new(0);
		#[cloned(inner1)]
		let _effect_inner1 = Effect::new(move || {
			_ = inner1.get();
			TIMES_RUN_INNER1.set(TIMES_RUN_INNER1.get() + 1);
		});

		// Then set it and ensure that `outer` was changed
		inner1.set(6);
		assert_eq!(outer.get(), Either2::T1(6));
		assert_eq!(TIMES_RUN_OUTER.get(), 2);
		assert_eq!(TIMES_RUN_INNER1.get(), 2);

		// Then change outer and get the `T2` signal
		outer.set(Either2::T2("a"));
		let Either2::T2(inner2) = signal.borrow() else {
			unreachable!("Signal was the wrong type at the beginning")
		};
		assert_eq!(TIMES_RUN_OUTER.get(), 3);
		assert_eq!(TIMES_RUN_INNER1.get(), 2);
		#[thread_local]
		static TIMES_RUN_INNER2: Cell<usize> = Cell::new(0);
		#[cloned(inner2)]
		let _effect_inner2 = Effect::new(move || {
			_ = inner2.get();
			TIMES_RUN_INNER2.set(TIMES_RUN_INNER2.get() + 1);
		});

		// Set `T2` to ensure changes are propagated
		inner2.set("b");
		assert_eq!(outer.get(), Either2::T2("b"));
		assert_eq!(TIMES_RUN_OUTER.get(), 4);
		assert_eq!(TIMES_RUN_INNER1.get(), 2);
		assert_eq!(TIMES_RUN_INNER2.get(), 2);

		// Set `T1` to ensure that signal changes are propagated
		inner1.set(7);
		assert_eq!(outer.get(), Either2::T1(7));
		assert_eq!(TIMES_RUN_OUTER.get(), 5);
		assert_eq!(TIMES_RUN_INNER1.get(), 3);
		assert_eq!(TIMES_RUN_INNER2.get(), 2);

		// Set `T2` from outer to ensure we get the same signal
		outer.set(Either2::T2("c"));
		assert_eq!(inner2.get(), "c");
		assert_eq!(TIMES_RUN_OUTER.get(), 6);
		assert_eq!(TIMES_RUN_INNER1.get(), 3);
		assert_eq!(TIMES_RUN_INNER2.get(), 3);
	}
}
