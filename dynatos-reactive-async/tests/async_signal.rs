//! [`AsyncSignal`] tests

// Features
#![feature(
	type_alias_impl_trait,
	proc_macro_hygiene,
	stmt_expr_attributes,
	async_fn_track_caller,
	decl_macro,
	unboxed_closures,
	never_type,
	async_fn_traits,
	impl_trait_in_assoc_type
)]

// Imports
use {
	core::{
		future::{self, Future},
		marker::PhantomPinned,
		pin::{pin, Pin},
		ptr,
		sync::atomic::{self, AtomicUsize},
		task::{self, Poll},
	},
	dynatos_reactive_async::AsyncSignal,
	zutil_cloned::cloned,
};

#[test]
fn async_signal_drop() {
	make_sig!(sig, CREATED, DROPPED);

	// When loading, after the first poll, it should be created, but not dropped
	let mut sig_load = pin!(sig.load());
	assert!(self::poll_once(sig_load.as_mut()).is_pending());
	assert_eq!(CREATED.load(atomic::Ordering::Acquire), 1);
	assert_eq!(DROPPED.load(atomic::Ordering::Acquire), 0);

	// Then after finishing, it should be dropped
	assert!(self::poll_once(sig_load).is_ready());
	assert_eq!(DROPPED.load(atomic::Ordering::Acquire), 1);

	// When loading again, no new future should be created
	let sig_load = pin!(sig.load());
	assert!(self::poll_once(sig_load).is_ready());
	assert_eq!(CREATED.load(atomic::Ordering::Acquire), 1);
}

#[test]
fn async_signal_drop_loading() {
	make_sig!(sig, CREATED, DROPPED);

	// Assert that creating, but not finishing loading creates the signal, but
	// does not drop it
	{
		let mut sig_load = sig.load();
		let mut sig_load = pin!(sig_load);
		assert!(self::poll_once(sig_load.as_mut()).is_pending());
		assert_eq!(CREATED.load(atomic::Ordering::Acquire), 1);
		assert_eq!(DROPPED.load(atomic::Ordering::Acquire), 0);
	};

	// Then by here it should be dropped, and trying to drop it should do nothing
	assert_eq!(DROPPED.load(atomic::Ordering::Acquire), 1);
	assert!(!sig.stop_loading(), "Signal still has the loader");
}

#[test]
fn async_signal_load() {
	type F = impl AsyncFnMut();

	let sig = AsyncSignal::<F>::new(|| async move {});

	let sig_load = pin!(sig.load());
	assert!(self::poll_once(sig_load).is_ready());
}

#[test]
fn async_signal_wait_unloaded() {
	type F = impl AsyncFnMut();

	let sig = AsyncSignal::<F>::new(|| async move {});

	let sig_wait = pin!(sig.wait());
	assert!(self::poll_once(sig_wait).is_pending());
}

#[test]
fn async_signal_load_dep() {
	type F0 = impl AsyncFnMut();
	type F1 = impl AsyncFnMut();

	let sig0 = AsyncSignal::<F0>::new(|| async move {});
	#[cloned(sig0)]
	let sig1 = AsyncSignal::<F1>::new(move || {
		let sig0 = sig0.clone();
		async move {
			sig0.load().await;
		}
	});

	let sig1_load = pin!(sig1.load());
	assert!(self::poll_once(sig1_load).is_ready());

	let sig0_wait = pin!(sig0.wait());
	assert!(self::poll_once(sig0_wait).is_ready());
}

#[test]
fn async_signal_load_continue() {
	type F0 = impl AsyncFnMut();
	type F1 = impl AsyncFnMut();

	let sig0 = AsyncSignal::<F0>::new(self::pending_once);

	#[cloned(sig0)]
	let sig1 = AsyncSignal::<F1>::new(move || {
		let sig0 = sig0.clone();
		async move {
			sig0.load().await;
		}
	});


	// Start `sig1` loading and run it to toggle the flag once
	let mut sig1_load = pin!(sig1.load());
	assert!(self::poll_once(sig1_load.as_mut()).is_pending());

	// Then clear `sig0` and ensure it doesn't finish
	assert!(sig0.stop_loading(), "sig0 loader was already dropped");
	assert!(self::poll_once(sig1_load.as_mut()).is_pending());
	assert!(self::poll_once(sig1_load.as_mut()).is_pending());

	// Then restart `sig0` and ensure `sig1` finishes.
	assert!(sig0.start_loading(), "Unable to start sig0 loader");
	assert!(self::poll_once(sig1_load.as_mut()).is_pending());
	assert!(self::poll_once(sig1_load).is_ready());
	assert!(self::poll_once(pin!(sig0.wait())).is_ready());
}

#[test]
fn async_signal_unpin() {
	type F = impl AsyncFnMut();

	#[pin_project::pin_project(PinnedDrop)]
	struct Fut {
		this:     *const Self,
		_phantom: PhantomPinned,
	}

	impl Future for Fut {
		type Output = ();

		fn poll(self: Pin<&mut Self>, _cx: &mut task::Context<'_>) -> Poll<Self::Output> {
			let self_ptr = &*self.as_ref() as *const Self;
			let this = self.project().this;

			if !this.is_null() {
				assert_eq!(*this, self_ptr);
			}

			*this = self_ptr;

			Poll::Ready(())
		}
	}

	#[pin_project::pinned_drop]
	impl PinnedDrop for Fut {
		fn drop(self: Pin<&mut Self>) {
			let self_ptr = &*self.as_ref() as *const Self;
			assert_eq!(self_ptr, *self.project().this);
		}
	}

	let sig = AsyncSignal::<F>::new(|| Fut {
		this:     ptr::null_mut(),
		_phantom: PhantomPinned,
	});

	let sig_load = pin!(sig.load());
	assert!(self::poll_once(sig_load).is_ready());
}

#[test]
fn async_signal_fn_mut() {
	struct F {
		num: usize,
	}

	impl AsyncFnMut<()> for F {
		type CallRefFuture<'a>
			= Fut<'a>
		where
			Self: 'a;

		extern "rust-call" fn async_call_mut(&mut self, _args: ()) -> Self::CallRefFuture<'_> {
			Fut { num: &mut self.num }
		}
	}

	impl AsyncFnOnce<()> for F {
		type Output = ();

		type CallOnceFuture = impl Future<Output = Self::Output>;

		extern "rust-call" fn async_call_once(self, (): ()) -> Self::CallOnceFuture {
			async move {}
		}
	}

	struct Fut<'a> {
		num: &'a mut usize,
	}

	impl Future for Fut<'_> {
		type Output = ();

		fn poll(self: Pin<&mut Self>, _cx: &mut task::Context<'_>) -> Poll<Self::Output> {
			Poll::Pending
		}
	}

	impl Drop for Fut<'_> {
		fn drop(&mut self) {
			*self.num += 1;
		}
	}

	let sig = AsyncSignal::<F>::new(F { num: 0 });
	sig.start_loading();
	assert!(self::poll_once(pin!(sig.wait())).is_pending());
	sig.restart_loading();
	assert!(self::poll_once(pin!(sig.wait())).is_pending());
}

/// Declares a new signal for testing drops.
macro make_sig($sig:ident, $CREATED:ident, $DROPPED:ident) {
	static $CREATED: AtomicUsize = AtomicUsize::new(0);
	static $DROPPED: AtomicUsize = AtomicUsize::new(0);

	type F = impl AsyncFnMut();
	let $sig = AsyncSignal::<F>::new(|| {
		$CREATED.fetch_add(1, atomic::Ordering::Release);
		async move {
			scopeguard::defer! {
				$DROPPED.fetch_add(1, atomic::Ordering::Release);
			}

			self::pending_once().await;
		}
	});
}

/// Returns a future that returns pending once, then becomes ready
async fn pending_once() {
	let mut pending = true;
	future::poll_fn(move |_cx| {
		let output = match pending {
			true => Poll::Pending,
			false => Poll::Ready(()),
		};
		pending = !pending;

		output
	})
	.await;
}

/// Polls a future once with a no-op waker
fn poll_once<F: Future>(fut: Pin<&mut F>) -> Poll<F::Output> {
	let mut cx = task::Context::from_waker(task::Waker::noop());
	fut.poll(&mut cx)
}
