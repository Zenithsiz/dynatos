//! Run queue

// Imports
use {
	super::ReactiveWorld,
	crate::{trigger::SubscriberInfo, Subscriber},
	core::{
		cell::LazyCell,
		cmp::Reverse,
		hash::{Hash, Hasher},
	},
	dynatos_world::{IMut, IMutLike, WorldGlobal, WorldThreadLocal},
	priority_queue::PriorityQueue,
	std::sync::LazyLock,
};

/// Run queue
// TODO: Require `W: ReactiveWorld` once that doesn't result in a cycle overflow.
pub trait RunQueue<W>: Sized {
	/// Exec guard
	type ExecGuard;

	/// Increases the reference count of the queue
	fn inc_ref();

	/// Decreases the reference count of the queue.
	///
	/// Returns a guard for execution all effects if
	/// this was the last trigger exec dropped.
	///
	/// Any further created trigger execs won't
	/// execute the functions and will instead just
	/// add them to the queue
	fn dec_ref() -> Option<Self::ExecGuard>;

	/// Pushes a subscriber to the queue.
	fn push(subscriber: Subscriber<W>, info: SubscriberInfo)
	where
		W: ReactiveWorld;

	/// Pops a subscriber from the front of the queue
	fn pop() -> Option<(Subscriber<W>, SubscriberInfo)>
	where
		W: ReactiveWorld;
}

/// Inner item for the priority queue
struct Item<W: ReactiveWorld> {
	/// Subscriber
	subscriber: Subscriber<W>,

	/// Info
	info: SubscriberInfo,
}

impl<W: ReactiveWorld> PartialEq for Item<W> {
	fn eq(&self, other: &Self) -> bool {
		self.subscriber == other.subscriber
	}
}

impl<W: ReactiveWorld> Eq for Item<W> {}

impl<W: ReactiveWorld> Hash for Item<W> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.subscriber.hash(state);
	}
}

/// Inner type for the queue impl
struct Inner<W: ReactiveWorld> {
	/// Queue
	// TODO: We don't need the priority, so just use some kind of
	//       `HashQueue`.
	queue: PriorityQueue<Item<W>, Reverse<usize>>,

	/// Next index
	next: usize,

	/// Reference count
	ref_count: usize,

	/// Whether currently executing the queue
	is_exec: bool,
}

impl<W: ReactiveWorld> Inner<W> {
	fn new() -> Self {
		Self {
			queue:     PriorityQueue::new(),
			next:      0,
			ref_count: 0,
			is_exec:   false,
		}
	}
}

/// Run queue impl
type RunQueueImpl<W> = IMut<Inner<W>, W>;

/// Thread-local run queue, using `StdRc` and `StdRefCell`
pub struct RunQueueThreadLocal;

/// Run queue for `RunQueueThreadLocal`
#[thread_local]
static RUN_QUEUE_STD_RC: LazyCell<RunQueueImpl<WorldThreadLocal>> =
	LazyCell::new(|| RunQueueImpl::<WorldThreadLocal>::new(Inner::new()));

/// `RunQueueThreadLocal` execution guard
pub struct RunQueueExecGuardThreadLocal;

impl Drop for RunQueueExecGuardThreadLocal {
	fn drop(&mut self) {
		let mut inner = RUN_QUEUE_STD_RC.write();
		inner.is_exec = false;
	}
}

impl RunQueue<WorldThreadLocal> for RunQueueThreadLocal {
	type ExecGuard = RunQueueExecGuardThreadLocal;

	fn inc_ref() {
		let mut inner = RUN_QUEUE_STD_RC.write();
		inner.ref_count += 1;
	}

	fn dec_ref() -> Option<Self::ExecGuard> {
		let mut inner = RUN_QUEUE_STD_RC.write();
		inner.ref_count = inner
			.ref_count
			.checked_sub(1)
			.expect("Attempted to decrease reference count beyond 0");

		(inner.ref_count == 0 && !inner.queue.is_empty() && !inner.is_exec).then(|| {
			inner.is_exec = true;
			RunQueueExecGuardThreadLocal
		})
	}

	fn push(subscriber: Subscriber<WorldThreadLocal>, info: SubscriberInfo) {
		let mut inner = RUN_QUEUE_STD_RC.write();

		let next = Reverse(inner.next);
		inner.queue.push_decrease(Item { subscriber, info }, next);
		inner.next += 1;
	}

	fn pop() -> Option<(Subscriber<WorldThreadLocal>, SubscriberInfo)>
	where
		WorldThreadLocal: ReactiveWorld,
	{
		let (item, _) = RUN_QUEUE_STD_RC.write().queue.pop()?;
		Some((item.subscriber, item.info))
	}
}

/// Global run queue, using `StdArc` and `StdRefCell`
pub struct RunQueueGlobal;

/// Run queue for `RunQueueGlobal`
static RUN_QUEUE_STD_ARC: LazyLock<RunQueueImpl<WorldGlobal>> =
	LazyLock::new(|| RunQueueImpl::<WorldGlobal>::new(Inner::new()));

/// `RunQueueGlobal` execution guard
pub struct RunQueueExecGuardGlobal;

impl Drop for RunQueueExecGuardGlobal {
	fn drop(&mut self) {
		let mut inner = RUN_QUEUE_STD_ARC.write();
		assert!(inner.is_exec, "Run queue stopped execution before guard was dropped");
		inner.is_exec = false;
	}
}

impl RunQueue<WorldGlobal> for RunQueueGlobal {
	type ExecGuard = RunQueueExecGuardGlobal;

	fn inc_ref() {
		let mut inner = RUN_QUEUE_STD_ARC.write();
		inner.ref_count += 1;
	}

	fn dec_ref() -> Option<Self::ExecGuard> {
		let mut inner = RUN_QUEUE_STD_ARC.write();
		inner.ref_count = inner
			.ref_count
			.checked_sub(1)
			.expect("Attempted to decrease reference count beyond 0");

		(inner.ref_count == 0 && !inner.queue.is_empty() && !inner.is_exec).then(|| {
			inner.is_exec = true;
			RunQueueExecGuardGlobal
		})
	}

	fn push(subscriber: Subscriber<WorldGlobal>, info: SubscriberInfo) {
		let mut inner = RUN_QUEUE_STD_ARC.write();

		let next = Reverse(inner.next);
		inner.queue.push_decrease(Item { subscriber, info }, next);
		inner.next += 1;
	}

	fn pop() -> Option<(Subscriber<WorldGlobal>, SubscriberInfo)>
	where
		WorldGlobal: ReactiveWorld,
	{
		let (item, _) = RUN_QUEUE_STD_ARC.write().queue.pop()?;
		Some((item.subscriber, item.info))
	}
}
