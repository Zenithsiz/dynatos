//! Trigger
//!
//! A reactivity primitive that allows re-running
//! any subscribers.

// Imports
#[cfg(debug_assertions)]
use core::panic::Location;
use {
	crate::{dep_graph, effect, run_queue},
	core::{
		cell::LazyCell,
		fmt,
		hash::{Hash, Hasher},
	},
	std::rc::{Rc, Weak},
};

/// Trigger inner
struct Inner {
	#[cfg(debug_assertions)]
	/// Where this trigger was defined
	defined_loc: &'static Location<'static>,
}

/// Trigger
pub struct Trigger {
	/// Inner
	inner: Rc<Inner>,
}

impl Trigger {
	/// Creates a new trigger
	#[must_use]
	#[track_caller]
	pub fn new() -> Self {
		let inner = Inner {
			#[cfg(debug_assertions)]
			defined_loc:                          Location::caller(),
		};
		Self { inner: Rc::new(inner) }
	}

	/// Downgrades this trigger
	#[must_use]
	pub fn downgrade(&self) -> WeakTrigger {
		WeakTrigger {
			inner: Rc::downgrade(&self.inner),
		}
	}

	/// Returns where this effect was defined
	#[cfg(debug_assertions)]
	pub(crate) fn defined_loc(&self) -> &'static Location<'static> {
		self.inner.defined_loc
	}

	/// Returns a unique identifier to this trigger.
	///
	/// Downgrading and cloning the trigger will retain the same id
	#[must_use]
	pub fn id(&self) -> usize {
		Rc::as_ptr(&self.inner).addr()
	}

	/// Gathers all effects depending on this trigger.
	///
	/// When triggering this trigger, all effects active during this gathering
	/// will be re-run.
	///
	/// You can gather multiple times without removing the previous gathered
	/// effects. Previous effects will only be removed when they are dropped.
	// TODO: Should we remove all existing subscribers before gathering them?
	#[track_caller]
	pub fn gather_subscribers(&self) {
		match effect::running() {
			Some(effect) => dep_graph::add_effect_dep(&effect, self),

			// TODO: Add some way to turn off this warning at a global
			//       scale, with something like
			//       `fn without_warning(f: impl FnOnce() -> O) -> O`
			#[cfg(debug_assertions)]
			None => tracing::warn!(
				trigger=?self,
				location=%Location::caller(),
				"No effect is being run when trigger was accessed. \
				\nThis typically means that you're accessing reactive \
				signals outside of an effect, which means the code won't \
				be re-run when the signal changes. If this is intention, \
				try to use one of the `_raw` methods that don't gather \
				subscribers to make it intentional"
			),

			#[cfg(not(debug_assertions))]
			None => (),
		}
	}

	/// Executes this trigger.
	///
	/// Adds all subscribers to the run queue, and once the returned
	/// executor is dropped, and there are no other executors alive,
	/// all queues effects are run.
	#[track_caller]
	#[expect(
		clippy::must_use_candidate,
		reason = "The user can just immediately drop the value to execute if they don't care"
	)]
	pub fn exec(&self) -> TriggerExec {
		self.exec_inner(
			#[cfg(debug_assertions)]
			Location::caller(),
		)
	}

	/// Creates an execution for a no-op trigger.
	///
	/// This is useful to ensure that another trigger
	/// doesn't execute the run queue and just appends to
	/// it instead.
	pub fn exec_noop() -> TriggerExec {
		/// No-op trigger
		#[thread_local]
		static NOOP_TRIGGER: LazyCell<Trigger> = LazyCell::new(Trigger::new);

		NOOP_TRIGGER.exec()
	}

	/// Inner function for [`Self::exec`]
	pub(crate) fn exec_inner(&self, #[cfg(debug_assertions)] caller_loc: &'static Location<'static>) -> TriggerExec {
		// If there's a running effect, register it as our dependency
		if let Some(effect) = effect::running() {
			dep_graph::add_effect_sub(
				&effect,
				self,
				#[cfg(debug_assertions)]
				caller_loc,
			);
		}

		// Increase the ref count
		run_queue::inc_ref();

		// Then add all subscribers to the run queue
		dep_graph::with_trigger_subs(self.downgrade(), |subscriber, subscriber_info| {
			// If the effect doesn't exist anymore, skip it
			let Some(effect) = subscriber.upgrade() else {
				return;
			};

			// Skip suppressed effects
			if effect.is_suppressed() {
				return;
			}

			// Then set the effect as stale and add it to the run queue
			effect.set_stale();
			run_queue::push(effect.downgrade(), subscriber_info);
		});

		TriggerExec {
			#[cfg(debug_assertions)]
			trigger_defined_loc:                          self.defined_loc(),
			#[cfg(debug_assertions)]
			exec_defined_loc:                             caller_loc,
		}
	}

	/// Formats this trigger into `s`
	fn fmt_debug(&self, mut s: fmt::DebugStruct<'_, '_>) -> Result<(), fmt::Error> {
		s.field("inner", &self.id());

		#[cfg(debug_assertions)]
		s.field_with("defined_loc", |f| fmt::Display::fmt(self.inner.defined_loc, f));

		s.finish()
	}
}

impl Default for Trigger {
	fn default() -> Self {
		Self::new()
	}
}

impl PartialEq for Trigger {
	fn eq(&self, other: &Self) -> bool {
		self.id() == other.id()
	}
}

impl Eq for Trigger {}


impl Clone for Trigger {
	fn clone(&self) -> Self {
		Self {
			inner: Rc::clone(&self.inner),
		}
	}
}

impl Hash for Trigger {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl fmt::Debug for Trigger {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.fmt_debug(f.debug_struct("Trigger"))
	}
}

/// Weak trigger
pub struct WeakTrigger {
	/// Inner
	inner: Weak<Inner>,
}

impl WeakTrigger {
	/// Creates an empty weak trigger
	#[must_use]
	pub const fn new() -> Self {
		Self { inner: Weak::new() }
	}

	/// Returns a unique identifier to this trigger.
	///
	/// Upgrading and cloning the trigger will retain the same id
	#[must_use]
	pub fn id(&self) -> usize {
		Weak::as_ptr(&self.inner).addr()
	}

	/// Upgrades this weak trigger
	#[must_use]
	pub fn upgrade(&self) -> Option<Trigger> {
		let inner = self.inner.upgrade()?;
		Some(Trigger { inner })
	}
}

impl Default for WeakTrigger {
	fn default() -> Self {
		Self::new()
	}
}

impl PartialEq for WeakTrigger {
	fn eq(&self, other: &Self) -> bool {
		self.id() == other.id()
	}
}

impl Eq for WeakTrigger {}

impl Clone for WeakTrigger {
	fn clone(&self) -> Self {
		Self {
			inner: Weak::clone(&self.inner),
		}
	}
}

impl Hash for WeakTrigger {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl fmt::Debug for WeakTrigger {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut s = f.debug_struct("WeakTrigger");

		match self.upgrade() {
			Some(trigger) => trigger.fmt_debug(s),
			None => s.finish_non_exhaustive(),
		}
	}
}

/// Trigger executor
pub struct TriggerExec {
	/// Trigger defined location
	#[cfg(debug_assertions)]
	trigger_defined_loc: &'static Location<'static>,

	/// Execution defined location
	// TODO: If a trigger gets executed inside of an effect,
	//       this location will point to this file (in the `Drop` impl),
	//       and we can't `#[track_caller]` past the drop impl, so this
	//       can be wrong.
	//       We can get a better guess by going to the dependency graph and
	//       getting the effect subscriber info, which will be where we're
	//       executed.
	#[cfg(debug_assertions)]
	exec_defined_loc: &'static Location<'static>,
}

impl Drop for TriggerExec {
	fn drop(&mut self) {
		// Decrease the reference count, and if we weren't the last, quit
		let Some(_exec_guard) = run_queue::dec_ref() else {
			return;
		};

		// If we were the last, keep popping effects and running them until
		// the run queue is empty
		while let Some((subscriber, info)) = run_queue::pop() {
			let Some(effect) = subscriber.upgrade() else {
				continue;
			};

			#[cfg(debug_assertions)]
			tracing::trace!(
				"Running effect due to trigger\nEffect   : {}\nGathered : {}\nTrigger  : {}\nExecution: {}",
				effect.defined_loc(),
				match info.is_empty() {
					true => "[]".to_owned(),
					#[expect(clippy::format_collect, reason = "TODO")]
					false => info
						.iter()
						.map(|info| format!("\n         - {}", info.gathered_loc))
						.collect::<String>(),
				},
				self.trigger_defined_loc,
				self.exec_defined_loc,
			);

			#[cfg(not(debug_assertions))]
			let _: Vec<_> = info;

			effect.run();
		}
	}
}

#[cfg(test)]
mod tests {
	// Imports
	extern crate test;
	use {
		super::*,
		crate::Effect,
		core::{array, cell::Cell, mem},
		test::Bencher,
		zutil_cloned::cloned,
	};

	#[test]
	fn basic() {
		/// Counts the number of times the effect was run
		#[thread_local]
		static TRIGGERS: Cell<usize> = Cell::new(0);

		let trigger = Trigger::new();
		#[cloned(trigger)]
		let effect = Effect::new(move || {
			trigger.gather_subscribers();
			TRIGGERS.set(TRIGGERS.get() + 1);
		});

		assert_eq!(TRIGGERS.get(), 1, "Trigger was triggered early");

		// Then trigger and ensure it was triggered
		trigger.exec();
		assert_eq!(TRIGGERS.get(), 2, "Trigger was not triggered");

		// Finally drop the effect and try again
		mem::drop(effect);
		trigger.exec();
		assert_eq!(TRIGGERS.get(), 2, "Trigger was triggered after effect was dropped");
	}

	#[test]
	fn exec_multiple() {
		/// Counts the number of times the effect was run
		#[thread_local]
		static TRIGGERS: Cell<usize> = Cell::new(0);

		let trigger = Trigger::new();
		#[cloned(trigger)]
		let _effect = Effect::new(move || {
			trigger.gather_subscribers();
			TRIGGERS.set(TRIGGERS.get() + 1);
		});

		let exec0 = trigger.exec();
		assert_eq!(TRIGGERS.get(), 1, "Trigger was triggered when executing");
		let exec1 = trigger.exec();

		drop(exec1);
		assert_eq!(
			TRIGGERS.get(),
			1,
			"Trigger was triggered when dropping a single executor"
		);

		drop(exec0);
		assert_eq!(
			TRIGGERS.get(),
			2,
			"Trigger wasn't triggered when dropping last executor"
		);
	}

	#[test]
	fn exec_multiple_same_effect() {
		/// Counts the number of times the effect was run
		#[thread_local]
		static TRIGGERS: Cell<usize> = Cell::new(0);

		let trigger0 = Trigger::new();
		let trigger1 = Trigger::new();
		#[cloned(trigger0, trigger1)]
		let _effect = Effect::new(move || {
			trigger0.gather_subscribers();
			trigger1.gather_subscribers();
			TRIGGERS.set(TRIGGERS.get() + 1);
		});

		let exec0 = trigger0.exec();
		let exec1 = trigger1.exec();

		drop((exec0, exec1));

		assert_eq!(TRIGGERS.get(), 2, "Effect was run multiple times in same run queue");

		trigger0.exec();

		assert_eq!(
			TRIGGERS.get(),
			3,
			"Effect wasn't run even when no other executors existed"
		);
	}

	#[bench]
	fn clone_100(bencher: &mut Bencher) {
		let triggers = array::from_fn::<Trigger, 100, _>(|_| Trigger::new());
		bencher.iter(|| {
			for trigger in &triggers {
				let trigger = test::black_box(trigger.clone());
				mem::forget(trigger);
			}
		});
	}

	/// Benches triggering a trigger with `N` no-op effects.
	fn trigger_noop_n<const N: usize>(bencher: &mut Bencher) {
		let trigger = Trigger::new();
		let _effects = array::from_fn::<_, N, _>(|_| {
			Effect::new(
				#[cloned(trigger)]
				move || trigger.gather_subscribers(),
			)
		});

		bencher.iter(|| {
			trigger.exec();
		});
	}

	#[bench]
	fn trigger_empty(bencher: &mut Bencher) {
		self::trigger_noop_n::<0>(bencher);
	}

	#[bench]
	fn trigger_noop(bencher: &mut Bencher) {
		self::trigger_noop_n::<1>(bencher);
	}

	#[bench]
	fn trigger_noop_10(bencher: &mut Bencher) {
		self::trigger_noop_n::<10>(bencher);
	}

	#[bench]
	fn trigger_noop_100(bencher: &mut Bencher) {
		self::trigger_noop_n::<100>(bencher);
	}

	#[bench]
	fn trigger_noop_1000(bencher: &mut Bencher) {
		self::trigger_noop_n::<1000>(bencher);
	}
}
