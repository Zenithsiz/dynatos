//! Run queue

// TODO: We should coordinate with the dependency graph to ensure we don't
//       run effects unnecessarily.

// Imports
use {
	crate::{dep_graph::EffectDepInfo, WeakEffect},
	core::{
		cell::{LazyCell, RefCell},
		cmp::Reverse,
		hash::{Hash, Hasher},
	},
	priority_queue::PriorityQueue,
};

/// Inner item for the priority queue
struct Item {
	/// Subscriber
	// TODO: Should the run queue use strong effects?
	sub: WeakEffect,

	/// Info
	info: Vec<EffectDepInfo>,
}

impl PartialEq for Item {
	fn eq(&self, other: &Self) -> bool {
		self.sub == other.sub
	}
}

impl Eq for Item {}

impl Hash for Item {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.sub.hash(state);
	}
}

/// Inner type for the queue impl
struct Inner {
	/// Queue
	// TODO: We don't need the priority, so just use some kind of
	//       `HashQueue`.
	queue: PriorityQueue<Item, Reverse<usize>>,

	/// Next index
	next: usize,

	/// Reference count
	ref_count: usize,

	/// Whether currently executing the queue
	is_exec: bool,
}

impl Inner {
	fn new() -> Self {
		Self {
			queue:     PriorityQueue::new(),
			next:      0,
			ref_count: 0,
			is_exec:   false,
		}
	}
}

/// Run queue
#[thread_local]
static RUN_QUEUE: LazyCell<RefCell<Inner>> = LazyCell::new(|| RefCell::new(Inner::new()));

/// Execution guard
pub struct ExecGuard;

impl Drop for ExecGuard {
	fn drop(&mut self) {
		let mut inner = RUN_QUEUE.borrow_mut();
		inner.is_exec = false;
	}
}

/// Increases the reference count of the queue
pub fn inc_ref() {
	let mut inner = RUN_QUEUE.borrow_mut();
	inner.ref_count += 1;
}

/// Decreases the reference count of the queue.
///
/// Returns a guard for execution all effects if
/// this was the last trigger exec dropped.
///
/// Any further created trigger execs won't
/// execute the functions and will instead just
/// add them to the queue
pub fn dec_ref() -> Option<ExecGuard> {
	let mut inner = RUN_QUEUE.borrow_mut();
	inner.ref_count = inner
		.ref_count
		.checked_sub(1)
		.expect("Attempted to decrease reference count beyond 0");

	(inner.ref_count == 0 && !inner.queue.is_empty() && !inner.is_exec).then(|| {
		inner.is_exec = true;
		ExecGuard
	})
}

/// Pushes a subscriber to the queue.
pub fn push(sub: WeakEffect, info: Vec<EffectDepInfo>) {
	let mut inner = RUN_QUEUE.borrow_mut();

	let next = Reverse(inner.next);
	inner.queue.push_decrease(Item { sub, info }, next);
	inner.next += 1;
}

/// Pops a subscriber from the front of the queue
pub fn pop() -> Option<(WeakEffect, Vec<EffectDepInfo>)> {
	let (item, _) = RUN_QUEUE.borrow_mut().queue.pop()?;
	Some((item.sub, item.info))
}
