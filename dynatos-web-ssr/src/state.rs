//! SSR State

// Imports
use {
	crate::{Document, History, HtmlBodyElement, HtmlHeadElement, Location, Window, event_target},
	core::{
		future,
		mem,
		task::{self, Waker},
	},
	std::sync::{Arc, nonpoison::Mutex},
};


#[derive(derive_more::Debug)]
pub(crate) struct Inner {
	pub(crate) wait_guards: Mutex<usize>,
	pub(crate) wait_wakers: Mutex<Vec<Waker>>,

	pub(crate) event_target_handlers: Mutex<Vec<event_target::Handler>>,

	pub(crate) history: History,
	pub(crate) window:  Window,
}

#[derive(Clone, Debug)]
pub struct State(pub(crate) Arc<Inner>);

impl State {
	#[must_use]
	pub fn new(location: String) -> Self {
		let window = Window::new(location);
		let history = History::new(window.document_ref().location_ref().clone());

		let inner = Inner {
			wait_guards: Mutex::new(0),
			wait_wakers: Mutex::new(vec![]),
			event_target_handlers: Mutex::new(vec![]),

			history,
			window,
		};
		Self(Arc::new(inner))
	}

	#[must_use]
	pub fn window(&self) -> &Window {
		&self.0.window
	}

	#[must_use]
	pub fn document(&self) -> &Document {
		self.window().document_ref()
	}

	#[must_use]
	pub fn head(&self) -> &HtmlHeadElement {
		self.document().head_ref()
	}

	#[must_use]
	pub fn body(&self) -> &HtmlBodyElement {
		self.document().body_ref()
	}

	#[must_use]
	pub fn history(&self) -> &History {
		&self.0.history
	}

	#[must_use]
	pub fn location(&self) -> &Location {
		self.document().location_ref()
	}

	#[must_use]
	pub fn wait_guard(&self) -> WaitGuard {
		WaitGuard::new(self.clone())
	}

	/// Waits for all wait guards to be dropped
	pub async fn wait_all(&self) {
		future::poll_fn(|ctx| {
			let wait_guards = *self.0.wait_guards.lock();
			match wait_guards == 0 {
				true => task::Poll::Ready(()),
				false => {
					self.0.wait_wakers.lock().push(ctx.waker().clone());
					task::Poll::Pending
				},
			}
		})
		.await;
	}
}

pub struct WaitGuard {
	state: State,
}

impl WaitGuard {
	#[must_use]
	pub fn new(state: State) -> Self {
		*state.0.wait_guards.lock() += 1;
		Self { state }
	}
}

impl Drop for WaitGuard {
	fn drop(&mut self) {
		let mut wait_guards = self.state.0.wait_guards.lock();
		*wait_guards -= 1;

		if *wait_guards == 0 {
			drop(wait_guards);

			let wakers = mem::take(&mut *self.state.0.wait_wakers.lock());
			for waker in wakers {
				waker.wake();
			}
		}
	}
}
