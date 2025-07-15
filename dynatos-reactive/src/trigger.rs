//! Trigger
//!
//! A reactivity primitive that allows re-running
//! any subscribers.

// Imports
use {
	crate::{effect, loc::Loc, WORLD},
	core::{
		cell::LazyCell,
		fmt,
		hash::{Hash, Hasher},
	},
	std::rc::{Rc, Weak},
};

/// Trigger inner
struct Inner {
	/// Where this trigger was defined
	defined_loc: Loc,
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
			defined_loc: Loc::caller(),
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
	pub(crate) fn defined_loc(&self) -> Loc {
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
	#[track_caller]
	pub fn gather_subs(&self) {
		// If the world is in "raw" mode, don't gather anything
		if WORLD.is_raw() {
			return;
		}

		match effect::running() {
			Some(effect) => WORLD.dep_graph().add_effect_dep(&effect, self),

			// TODO: Add some way to turn off this warning at a global
			//       scale, with something like
			//       `fn without_warning(f: impl FnOnce() -> O) -> O`
			None => tracing::warn!(
				trigger=?self,
				location=%Loc::caller(),
				"No effect is being run when trigger was accessed. \
				\nThis typically means that you're accessing reactive \
				signals outside of an effect, which means the code won't \
				be re-run when the signal changes. If this is intention, \
				try to use one of the `_raw` methods that don't gather \
				subscribers to make it intentional"
			),
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
	pub fn exec(&self) -> Option<TriggerExec> {
		self.exec_inner(Loc::caller())
	}

	/// Creates an execution for a no-op trigger.
	///
	/// This is useful to ensure that another trigger
	/// doesn't execute the run queue and just appends to
	/// it instead.
	pub fn exec_noop() -> Option<TriggerExec> {
		/// No-op trigger
		#[thread_local]
		static NOOP_TRIGGER: LazyCell<Trigger> = LazyCell::new(Trigger::new);

		NOOP_TRIGGER.exec()
	}

	/// Inner function for [`Self::exec`]
	pub(crate) fn exec_inner(&self, caller_loc: Loc) -> Option<TriggerExec> {
		// If the world is in "raw" mode, don't execute anything
		// TODO: Should we still return just a `TriggerExec`, but make
		//       it not do anything on drop?
		if WORLD.is_raw() {
			return None;
		}

		// If there's a running effect, register it as our dependency
		if let Some(effect) = effect::running() {
			WORLD.dep_graph().add_effect_sub(&effect, self, caller_loc);
		}

		// Increase the ref count
		WORLD.run_queue().inc_ref();

		// Then add all subscribers to the run queue
		WORLD.dep_graph().with_trigger_subs(self.downgrade(), |sub, sub_info| {
			// If the effect doesn't exist anymore, skip it
			let Some(effect) = sub.upgrade() else {
				return;
			};

			// Skip suppressed effects
			if effect.is_suppressed() {
				return;
			}

			// Then set the effect as stale and add it to the run queue
			effect.set_stale();
			WORLD.run_queue().push(effect.downgrade(), sub_info);
		});

		Some(TriggerExec {
			trigger_defined_loc: self.defined_loc(),
			exec_defined_loc:    caller_loc,
		})
	}

	/// Formats this trigger into `s`
	#[coverage(off)]
	fn fmt_debug(&self, mut s: fmt::DebugStruct<'_, '_>) -> Result<(), fmt::Error> {
		s.field("id", &self.id());

		s.field("defined_loc", &self.defined_loc());

		s.finish()
	}
}

#[coverage(off)]
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

#[coverage(off)]
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

#[coverage(off)]
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

#[coverage(off)]
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
	trigger_defined_loc: Loc,

	/// Execution defined location
	// TODO: If a trigger gets executed inside of an effect,
	//       this location will point to this file (in the `Drop` impl),
	//       and we can't `#[track_caller]` past the drop impl, so this
	//       can be wrong.
	//       We can get a better guess by going to the dependency graph and
	//       getting the effect subscriber info, which will be where we're
	//       executed.
	exec_defined_loc: Loc,
}

impl Drop for TriggerExec {
	fn drop(&mut self) {
		// Decrease the reference count, and if we weren't the last, quit
		let Some(_exec_guard) = WORLD.run_queue().dec_ref() else {
			return;
		};

		// If we were the last, keep popping effects and running them until
		// the run queue is empty
		while let Some((sub, sub_info)) = WORLD.run_queue().pop() {
			let Some(effect) = sub.upgrade() else {
				continue;
			};

			tracing::trace!(
				"Running effect due to trigger\nEffect   : {}\nGathered : {}\nTrigger  : {}\nExecution: {}",
				effect.defined_loc(),
				match sub_info.is_empty() {
					true => "[]".to_owned(),
					#[expect(clippy::format_collect, reason = "TODO")]
					false => sub_info
						.iter()
						.map(|info| format!("\n         - {}", info.gathered_loc))
						.collect::<String>(),
				},
				self.trigger_defined_loc,
				self.exec_defined_loc,
			);

			effect.run();
		}
	}
}
