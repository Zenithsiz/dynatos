//! Run queue tests

// Features
#![feature(thread_local, proc_macro_hygiene, stmt_expr_attributes)]

// Imports
use {
	core::cell::{Cell, RefCell},
	dynatos_reactive::{Derived, Effect, Signal, SignalBorrowMut, SignalGet, Trigger},
	zutil_cloned::cloned,
};

/// Ensures that the run queue is breadth-first
#[test]
fn breadth_first() {
	let a = Signal::new(5_usize);
	let b = Signal::new(6_usize);

	#[thread_local]
	static ORDER: RefCell<Vec<&'static str>> = RefCell::new(vec![]);

	#[cloned(a)]
	let a2 = Derived::new(move || {
		ORDER.borrow_mut().push("a2");
		a.get() + 1
	});
	#[cloned(b)]
	let b2 = Derived::new(move || {
		ORDER.borrow_mut().push("b2");
		b.get() + 1
	});

	let _c = Effect::new(move || {
		ORDER.borrow_mut().push("c");
		_ = (a2.get(), b2.get());
	});

	let a = a.borrow_mut();
	let b = b.borrow_mut();

	ORDER.borrow_mut().clear();
	drop((a, b));
	assert_eq!(*ORDER.borrow(), ["a2", "b2", "c"], "Effect was run with wrong order");
}

#[test]
fn order() {
	// a1╶─🭬a2╶─┬─🭬c
	//       b╶─┘
	let a1 = Trigger::new();
	let a2 = Trigger::new();
	let b = Trigger::new();

	#[cloned(a1, a2)]
	let _a1_to_2 = Effect::new(move || {
		a1.gather_subscribers();
		a2.exec();
	});

	#[thread_local]
	static COUNT: Cell<usize> = Cell::new(0);

	#[cloned(b)]
	let _c = Effect::new(move || {
		a2.gather_subscribers();
		b.gather_subscribers();
		COUNT.set(COUNT.get() + 1);
	});

	assert_eq!(COUNT.get(), 1);

	drop((a1.exec(), b.exec()));
	assert_eq!(COUNT.get(), 2);

	// TODO: Can we make this `3` by creating
	//       a full dependency graph?
	drop((b.exec(), a1.exec()));
	assert_eq!(COUNT.get(), 4);
}
