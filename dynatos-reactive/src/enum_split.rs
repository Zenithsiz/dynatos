//! Enum split signal

// Modules
mod ctx;
mod either;
mod storage;

// Exports
pub use self::{
	ctx::EnumSplitValueUpdateCtx,
	either::{All1, All2, All3, Either1, Either2, Either3},
	storage::SignalStorage,
};

// Imports
use {
	crate::{
		Effect,
		EffectRun,
		EffectRunCtx,
		Signal,
		SignalBorrow,
		SignalGetCloned,
		SignalGetClonedDefaultImpl,
		SignalGetDefaultImpl,
		SignalSet,
		SignalWithDefaultImpl,
		Trigger,
	},
	core::{cell::RefCell, fmt},
};

/// Enum split signal
pub struct EnumSplitSignal<S, T: EnumSplitValue<S>> {
	/// Effect
	effect: Effect<EffectFn<S, T>>,
}

impl<S, T: EnumSplitValue<S> + 'static> EnumSplitSignal<S, T> {
	/// Creates a new enum split signal
	pub fn new(signal: S) -> Self
	where
		T: 'static,
		S: SignalGetCloned<Value = T> + SignalSet<T> + Clone + 'static,
	{
		let effect = Effect::new(EffectFn {
			inner: RefCell::new(EffectFnInner::default()),
			trigger: Trigger::new(),
			signal,
		});

		Self { effect }
	}

	/// Converts this signal into an effect.
	// TODO: This only serves to keep effects alive in html nodes,
	//       can we simply do that some other way?
	#[must_use]
	pub fn into_effect(self) -> Effect<impl EffectRun>
	where
		S: SignalGetCloned<Value = T> + SignalSet<T> + Clone + 'static,
	{
		self.effect
	}
}

impl<S, T: EnumSplitValue<S>> Clone for EnumSplitSignal<S, T> {
	fn clone(&self) -> Self {
		Self {
			effect: self.effect.clone(),
		}
	}
}

#[coverage(off)]
impl<S, T> fmt::Debug for EnumSplitSignal<S, T>
where
	T: EnumSplitValue<S>,
	T::SignalsStorage: fmt::Debug,
	T::SigKind: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut s = f.debug_struct("EnumSplitSignal");
		s.field("effect", &self.effect);

		match self.effect.inner_fn().inner.try_borrow() {
			Ok(inner) => s
				.field("signals", &inner.signals)
				.field("cur_kind", &inner.cur_kind)
				.finish(),
			Err(_) => s.finish_non_exhaustive(),
		}
	}
}

impl<S, T: EnumSplitValue<S>> SignalBorrow for EnumSplitSignal<S, T> {
	type Ref<'a>
		= T::Signal
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		self.effect.inner_fn().trigger.gather_subs();
		let effect_fn = self.effect.inner_fn();

		let inner = effect_fn.inner.borrow();

		let cur = inner.cur_kind.as_ref().expect("Should have a current signal");
		T::get_signal(&inner.signals, cur).expect("Signal for current signal was missing")
	}
}

impl<S, T: EnumSplitValue<S>> SignalGetCloned for EnumSplitSignal<S, T> {
	type Value = T::Signal;

	fn get_cloned(&self) -> Self::Value {
		self.borrow()
	}
}

impl<S, T: EnumSplitValue<S>> SignalWithDefaultImpl for EnumSplitSignal<S, T> {}

// Note: Since our `Borrow` impl doesn't return a reference, we implement
//       `GetCloned` manually, so we don't want the default impl
impl<S, T: EnumSplitValue<S>> !SignalGetDefaultImpl for EnumSplitSignal<S, T> {}
impl<S, T: EnumSplitValue<S>> !SignalGetClonedDefaultImpl for EnumSplitSignal<S, T> {}

/// Effect fn inner
struct EffectFnInner<S, T: EnumSplitValue<S>> {
	/// Signals
	signals: T::SignalsStorage,

	/// Current signal kind
	cur_kind: Option<T::SigKind>,
}

impl<S, T: EnumSplitValue<S>> Default for EffectFnInner<S, T> {
	fn default() -> Self {
		Self {
			signals:  T::SignalsStorage::default(),
			cur_kind: None,
		}
	}
}

/// Inner effect function
struct EffectFn<S, T: EnumSplitValue<S>> {
	/// Inner
	inner: RefCell<EffectFnInner<S, T>>,

	/// Trigger
	trigger: Trigger,

	/// Signal
	signal: S,
}

impl<S, T> EffectRun for EffectFn<S, T>
where
	T: EnumSplitValue<S> + 'static,
	S: SignalGetCloned<Value = T> + SignalSet<T> + Clone + 'static,
{
	crate::effect_run_impl_inner! {}

	fn run(&self, _run_ctx: EffectRunCtx<'_>) {
		// Get the new value
		let new_value = self.signal.get_cloned();

		// Then update the current signal
		let mut inner = self.inner.borrow_mut();
		let prev_kind = inner.cur_kind.replace(new_value.kind());
		let update_ctx = EnumSplitValueUpdateCtx::new(self.signal.clone());
		new_value.update(&mut inner.signals, update_ctx);

		if prev_kind != inner.cur_kind {
			drop(inner);
			self.trigger.exec();
		}
	}
}

/// Enum split value
pub trait EnumSplitValue<S> {
	/// Signals storage
	type SignalsStorage: Default;

	/// Signal type
	type Signal;

	/// Signal kind
	type SigKind: PartialEq + fmt::Debug;

	/// Extracts a signal from storage
	fn get_signal(storage: &Self::SignalsStorage, kind: &Self::SigKind) -> Option<Self::Signal>;

	/// Gets the signal kind of this value
	fn kind(&self) -> Self::SigKind;

	/// Updates a signal with this value
	fn update(self, storage: &mut Self::SignalsStorage, ctx: EnumSplitValueUpdateCtx<'_, S>);
}

impl<S, T> EnumSplitValue<S> for Option<T>
where
	T: Clone + 'static,
	S: SignalSet<Self> + Clone + 'static,
{
	type SigKind = Option<()>;
	type Signal = Option<Signal<T>>;
	type SignalsStorage = Option<SignalStorage<T>>;

	fn get_signal(storage: &Self::SignalsStorage, kind: &Self::SigKind) -> Option<Self::Signal> {
		let signal = match kind {
			Some(()) => Some(storage.as_ref()?.signal()),
			None => None,
		};

		Some(signal)
	}

	fn kind(&self) -> Self::SigKind {
		self.as_ref().map(|_| ())
	}

	fn update(self, storage: &mut Self::SignalsStorage, ctx: EnumSplitValueUpdateCtx<'_, S>) {
		let Some(new_value) = self else { return };

		match storage {
			Some(storage) => storage.set(new_value),
			None => *storage = Some(ctx.create_signal_storage(new_value, Some)),
		}
	}
}

/// Extension trait to create an enum split signal
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
		let input = Signal::new(Either2::<usize, ()>::T2(()));

		#[cloned(input)]
		let signal = EnumSplitSignal::new(input);

		#[thread_local]
		static EFFECT_SOME: OnceCell<Effect> = OnceCell::new();

		#[thread_local]
		static EFFECT_NONE: OnceCell<Effect> = OnceCell::new();

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
		let outer = Signal::new(Either2::<usize, &'static str>::T1(5));
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
