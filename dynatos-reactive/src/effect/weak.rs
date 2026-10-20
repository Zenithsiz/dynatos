//! Weak effect

use {
	super::{Effect, EffectRun, Inner},
	core::{
		fmt,
		hash::{Hash, Hasher},
		marker::Unsize,
		ops::CoerceUnsized,
	},
	std::rc::{Rc, Weak},
};

/// Weak effect
///
/// Used to break ownership between a signal and it's subscribers
pub struct WeakEffect<F: ?Sized = dyn EffectRun> {
	/// Inner
	pub(super) inner: Weak<Inner<F>>,
}

impl<F> WeakEffect<F> {
	/// Creates an empty weak effect
	#[must_use]
	pub const fn new() -> Self {
		Self { inner: Weak::new() }
	}
}

impl<F: ?Sized> WeakEffect<F> {
	/// Upgrades this effect
	#[must_use]
	pub fn upgrade(&self) -> Option<Effect<F>> {
		self.inner.upgrade().map(|inner| Effect { inner })
	}

	/// Returns a unique identifier to this effect.
	///
	/// Upgrading and cloning the effect will retain the same id
	#[must_use]
	pub fn id(&self) -> usize {
		Weak::as_ptr(&self.inner).addr()
	}

	/// Runs this effect, if it exists.
	///
	/// Returns if the effect still existed
	#[track_caller]
	#[expect(
		clippy::must_use_candidate,
		reason = "The user may not care whether we actually ran or not"
	)]
	pub fn try_run(&self) -> bool
	where
		F: EffectRun + 'static,
	{
		// Try to upgrade, else return that it was missing
		let Some(effect) = self.upgrade() else {
			return false;
		};

		effect.run();
		true
	}

	/// Unsizes this value into a `WeakEffect`.
	// Note: This is necessary for unsizing from `!Sized` to `dyn EffectRun`,
	//       since those coercions only work for `Sized` types.
	// TODO: Once we can unsize from `?Sized` to `dyn EffectRun`,
	//       remove this.
	#[must_use]
	pub fn unsize(&self) -> WeakEffect
	where
		F: EffectRun,
	{
		// Note: We can't call `unsize_inner` on a `Weak`, so
		//       we need to first upgrade and call it that way.
		match self.inner.upgrade() {
			Some(inner) => WeakEffect {
				inner: Rc::downgrade(&inner.unsize_inner()),
			},

			// Note: If we failed upgrading, we simply create a `Weak`
			//       that can never be upgraded. This is technically a
			//       breaking change, since the weak count will be different,
			//       but we don't care about that for now.
			// TODO: For effects created with `WeakEffect::new`, this will result
			//       in another effect that actually compares equal, but for others
			//       it won't.
			None => WeakEffect {
				inner: Weak::<Inner<!>>::new(),
			},
		}
	}
}

#[coverage(off)]
impl<F> Default for WeakEffect<F> {
	fn default() -> Self {
		Self::new()
	}
}

impl<F1: ?Sized, F2: ?Sized> PartialEq<WeakEffect<F2>> for WeakEffect<F1> {
	fn eq(&self, other: &WeakEffect<F2>) -> bool {
		self.id() == other.id()
	}
}

impl<F: ?Sized> Eq for WeakEffect<F> {}

impl<F: ?Sized> Clone for WeakEffect<F> {
	fn clone(&self) -> Self {
		Self {
			inner: Weak::clone(&self.inner),
		}
	}
}


impl<F: ?Sized> Hash for WeakEffect<F> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

#[coverage(off)]
impl<F: ?Sized> fmt::Debug for WeakEffect<F> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut s = f.debug_struct("WeakEffect");

		match self.upgrade() {
			Some(effect) => effect.fmt_debug(s),
			None => s.finish_non_exhaustive(),
		}
	}
}

impl<F1, F2> CoerceUnsized<WeakEffect<F2>> for WeakEffect<F1>
where
	F1: ?Sized + Unsize<F2>,
	F2: ?Sized,
{
}
