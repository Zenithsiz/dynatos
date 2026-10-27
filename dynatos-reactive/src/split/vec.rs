//! Vec signal split

use {
	crate::{
		Effect,
		EffectRun,
		SignalBorrow,
		SignalBorrowMut,
		SignalGetClonedDefaultImpl,
		SignalGetDefaultImpl,
		SignalSetDefaultImpl,
		SignalUpdateDefaultImpl,
		SignalWithDefaultImpl,
		Trigger,
		effect::EffectSuppressed,
		trigger::TriggerExec,
	},
	core::{
		cell::{self, RefCell},
		clone::Share,
		ops::{self, Deref, DerefMut},
	},
	std::rc::Rc,
	zutil_cloned::cloned,
};

#[derive(Debug)]
struct Output<O> {
	value:   Option<O>,
	trigger: Trigger,
}

#[derive(Debug)]
struct Inner<O> {
	output:         RefCell<Vec<Output<O>>>,
	update_proxies: Effect,
}

/// Vec signal split
///
/// Allows splitting a signal with a `Vec` value into
/// a `Vec` of signals.
#[derive(Debug)]
pub struct VecSplitSignal<O> {
	inner: Rc<Inner<O>>,
}

impl<O> VecSplitSignal<O> {
	pub fn new<S, F>(signal: S, f: F) -> Self
	where
		O: 'static,
		S: Clone + for<'a> SignalBorrow<Ref<'a>: Deref<Target: VecSplit>> + 'static,
		F: Fn(VecSplitSignalProxy<S, O>) -> O + 'static,
	{
		let inner = Rc::<Inner<O>>::new_cyclic(move |inner| {
			#[cloned(inner)]
			let update_proxies = Effect::new_raw(move || {
				let Some(inner) = inner.upgrade() else { return };
				let len = signal.borrow().as_slice().len();

				let mut output = inner.output.borrow_mut();
				let mut cur_idx = 0;
				while let Some(value) = output.get(cur_idx) {
					let exec = value.trigger.exec();
					drop(output);
					drop(exec);
					output = inner.output.borrow_mut();
					cur_idx += 1;
				}

				match len <= output.len() {
					true => output.truncate(len),
					false =>
					// Note: We need to do it like this because we can't hold the lock while
					//       calling `f`, and while this effect should never recurse while
					//       holding the lock, we still guard against it by looping one element
					//       at a time.
						while len > output.len() {
							let idx = output.len();
							output.push(Output {
								value:   None,
								trigger: Trigger::new(),
							});

							let proxy = VecSplitSignalProxy {
								inner: inner.share(),
								signal: signal.clone(),
								idx,
							};
							drop(output);
							let value = f(proxy);

							output = inner.output.borrow_mut();
							output[idx].value = Some(value);
						},
				}
			});

			Inner {
				output: RefCell::new(vec![]),
				update_proxies,
			}
		});

		// Note: Needs to run outside because the `Inner` isn't constructed yet.
		inner.update_proxies.run();

		Self { inner }
	}
}

impl<O> SignalBorrow for VecSplitSignal<O> {
	type Ref<'a>
		= VecSplitSignalBorrow<'a, O>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		VecSplitSignalBorrow(self.inner.output.borrow())
	}
}

#[derive(Debug)]
pub struct VecSplitSignalBorrow<'a, O>(cell::Ref<'a, Vec<Output<O>>>);

impl<O> VecSplitSignalBorrow<'_, O> {
	pub fn iter(&self) -> impl Iterator<Item = &O> {
		self.0
			.iter()
			.map(|output| output.value.as_ref().expect("Value was missing"))
	}
}

#[derive(Clone, Debug)]
pub struct VecSplitSignalProxy<S, O> {
	inner:  Rc<Inner<O>>,
	signal: S,
	idx:    usize,
}

impl<S, O> SignalBorrow for VecSplitSignalProxy<S, O>
where
	S: SignalBorrow,
{
	type Ref<'a>
		= VecSplitSignalProxyBorrow<'a, S>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		self.inner.output.borrow()[self.idx].trigger.gather_subs();

		VecSplitSignalProxyBorrow {
			borrow: self.signal.borrow_no_dep(),
			idx:    self.idx,
		}
	}
}

impl<S, O> SignalBorrowMut for VecSplitSignalProxy<S, O>
where
	S: SignalBorrowMut,
{
	type RefMut<'a>
		= VecSplitSignalProxyBorrowMut<'a, S>
	where
		Self: 'a;

	fn borrow_mut(&self) -> Self::RefMut<'_> {
		let exec = self.inner.output.borrow()[self.idx].trigger.exec();

		VecSplitSignalProxyBorrowMut {
			_suppress: self.inner.update_proxies.suppress(),
			borrow:    self.signal.borrow_mut(),
			_exec:     exec,
			idx:       self.idx,
		}
	}
}

impl<S, O> SignalGetDefaultImpl for VecSplitSignalProxy<S, O> {}
impl<S, O> SignalGetClonedDefaultImpl for VecSplitSignalProxy<S, O> {}
impl<S, O> SignalWithDefaultImpl for VecSplitSignalProxy<S, O> {}

impl<S, O> SignalSetDefaultImpl for VecSplitSignalProxy<S, O> {}
impl<S, O> SignalUpdateDefaultImpl for VecSplitSignalProxy<S, O> {}

#[derive(Debug)]
pub struct VecSplitSignalProxyBorrow<'a, S>
where
	S: SignalBorrow + 'a,
{
	borrow: S::Ref<'a>,
	idx:    usize,
}

impl<'a, S> ops::Deref for VecSplitSignalProxyBorrow<'a, S>
where
	S: SignalBorrow<Ref<'a>: Deref<Target: VecSplit>>,
{
	type Target = <<S::Ref<'a> as Deref>::Target as VecSplit>::T;

	fn deref(&self) -> &Self::Target {
		&self.borrow.as_slice()[self.idx]
	}
}

#[derive(Debug)]
pub struct VecSplitSignalProxyBorrowMut<'a, S>
where
	S: SignalBorrowMut + 'a,
{
	_suppress: EffectSuppressed<'a, dyn EffectRun>,
	borrow:    S::RefMut<'a>,
	_exec:     Option<TriggerExec>,
	idx:       usize,
}

impl<'a, S> ops::Deref for VecSplitSignalProxyBorrowMut<'a, S>
where
	S: SignalBorrowMut<RefMut<'a>: Deref<Target: VecSplit>>,
{
	type Target = <<S::RefMut<'a> as Deref>::Target as VecSplit>::T;

	fn deref(&self) -> &Self::Target {
		&self.borrow.as_slice()[self.idx]
	}
}

impl<'a, S> ops::DerefMut for VecSplitSignalProxyBorrowMut<'a, S>
where
	S: SignalBorrowMut<RefMut<'a>: DerefMut<Target: VecSplitMut>>,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.borrow.as_slice_mut()[self.idx]
	}
}

pub trait VecSplit {
	type T: 'static;

	fn as_slice(&self) -> &[Self::T];
}

pub trait VecSplitMut: VecSplit {
	fn as_slice_mut(&mut self) -> &mut [Self::T];
}

impl<V> VecSplit for &'_ V
where
	V: VecSplit,
{
	type T = V::T;

	fn as_slice(&self) -> &[Self::T] {
		(**self).as_slice()
	}
}

impl<V> VecSplit for &'_ mut V
where
	V: VecSplit,
{
	type T = V::T;

	fn as_slice(&self) -> &[Self::T] {
		(**self).as_slice()
	}
}

impl<V> VecSplitMut for &'_ mut V
where
	V: VecSplitMut,
{
	fn as_slice_mut(&mut self) -> &mut [Self::T] {
		(**self).as_slice_mut()
	}
}

impl<T: 'static> VecSplit for Vec<T> {
	type T = T;

	fn as_slice(&self) -> &[Self::T] {
		self
	}
}

impl<T: 'static> VecSplitMut for Vec<T> {
	fn as_slice_mut(&mut self) -> &mut [Self::T] {
		self
	}
}

#[cfg(test)]
mod tests {
	use {
		super::*,
		crate::{Derived, Signal, SignalBorrowMut, SignalGet, SignalSet},
		core::cell::Cell,
	};


	#[test]
	fn basic() {
		let signal = Signal::new(vec![1, 2, 3]);

		let outer_derived_updated = Rc::new(Cell::new(0_usize));

		#[cloned(signal, outer_derived_updated)]
		let get_values = Derived::new(move || {
			outer_derived_updated.update(|value| value + 1);
			signal.borrow().iter().copied().collect::<Vec<_>>()
		});

		let inner_derived_updated = Rc::new(Cell::new(0_usize));
		let inner_proxies = Rc::new(RefCell::new(vec![]));

		#[cloned(inner_derived_updated, inner_proxies)]
		let split_signal = VecSplitSignal::new(signal.clone(), move |proxy| {
			inner_proxies.borrow_mut().push(proxy.clone());

			#[cloned(inner_derived_updated)]
			Derived::new(move || {
				inner_derived_updated.update(|value| value + 1);
				proxy.get() + 2
			})
			.unsize()
		});


		let get_derived_values = move || {
			split_signal
				.borrow()
				.iter()
				.map(|derived| derived.get())
				.collect::<Vec<_>>()
		};

		assert_eq!(outer_derived_updated.get(), 1);
		assert_eq!(inner_derived_updated.get(), 3);
		assert_eq!(inner_proxies.borrow().len(), 3);
		assert_eq!(&**get_values.borrow(), [1, 2, 3]);
		assert_eq!(get_derived_values(), [3, 4, 5]);

		// Updating through the original signal must update all derived without
		// re-creating them.
		signal.borrow_mut()[0] = 4;
		assert_eq!(outer_derived_updated.get(), 2);
		assert_eq!(inner_derived_updated.get(), 6);
		assert_eq!(inner_proxies.borrow().len(), 3);
		assert_eq!(&**get_values.borrow(), [4, 2, 3]);
		assert_eq!(get_derived_values(), [6, 4, 5]);

		// Updating through the proxies must only update the single derived and other dependencies.
		inner_proxies.borrow()[0].set(7);
		assert_eq!(outer_derived_updated.get(), 3);
		assert_eq!(inner_derived_updated.get(), 7);
		assert_eq!(inner_proxies.borrow().len(), 3);
		assert_eq!(&**get_values.borrow(), [7, 2, 3]);
		assert_eq!(get_derived_values(), [9, 4, 5]);

		// Adding a new element should only create a single output
		signal.borrow_mut().push(0);
		assert_eq!(outer_derived_updated.get(), 4);
		assert_eq!(inner_derived_updated.get(), 11);
		assert_eq!(inner_proxies.borrow().len(), 4);
		assert_eq!(&**get_values.borrow(), [7, 2, 3, 0]);
		assert_eq!(get_derived_values(), [9, 4, 5, 2]);

		// Adding a new element should only update 3 outputs
		signal.borrow_mut().pop().expect("Should exist");
		assert_eq!(outer_derived_updated.get(), 5);
		assert_eq!(inner_derived_updated.get(), 14);
		assert_eq!(inner_proxies.borrow().len(), 4); // Note: We don't auto-remove the proxies
		assert_eq!(&**get_values.borrow(), [7, 2, 3]);
		assert_eq!(get_derived_values(), [9, 4, 5]);

		// Updating both the original and a proxy should update everything once, regardless of order
		{
			let dummy_trigger = Trigger::new();
			let _exec = dummy_trigger.exec();

			signal.borrow_mut()[0] = 1;
			inner_proxies.borrow()[1].set(3);
		}
		assert_eq!(outer_derived_updated.get(), 6);
		assert_eq!(inner_derived_updated.get(), 17);
		assert_eq!(inner_proxies.borrow().len(), 4);
		assert_eq!(&**get_values.borrow(), [1, 3, 3]);
		assert_eq!(get_derived_values(), [3, 5, 5]);
		{
			let dummy_trigger = Trigger::new();
			let _exec = dummy_trigger.exec();

			inner_proxies.borrow()[1].set(5);
			signal.borrow_mut()[0] = 2;
		}
		assert_eq!(outer_derived_updated.get(), 7);
		assert_eq!(inner_derived_updated.get(), 20);
		assert_eq!(inner_proxies.borrow().len(), 4);
		assert_eq!(&**get_values.borrow(), [2, 5, 3]);
		assert_eq!(get_derived_values(), [4, 7, 5]);
	}
}
