//! Run queue tests

// Features
#![feature(thread_local, proc_macro_hygiene, stmt_expr_attributes, array_windows)]

// Imports
use {
	core::{
		cell::{Cell, RefCell},
		iter,
	},
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
fn multiple() {
	let a = Trigger::new();

	#[thread_local]
	static COUNT: Cell<usize> = Cell::new(0);
	#[cloned(a)]
	let _effect = Effect::new(move || {
		a.gather_subscribers();
		a.gather_subscribers();
		COUNT.set(COUNT.get() + 1);
	});

	assert_eq!(COUNT.get(), 1);
	a.exec();
	assert_eq!(COUNT.get(), 2, "Effect was run multiple times");
}

#[test]
fn order() {
	// a1╶─🭬a2╶─🭬aN-1╶─🭬aN╶─┬─🭬c
	//                  b╶──┘
	let a = iter::repeat_with(|| Trigger::new()).take(3).collect::<Vec<_>>();
	let b = Trigger::new();

	let a_first = a.first().expect("Empty `a`s").clone();
	let a_last = a.last().expect("Empty `a`s").clone();

	#[expect(clippy::redundant_clone, reason = "False positive")]
	let _a_effects = a
		.array_windows()
		.cloned()
		.map(move |[lhs, rhs]| {
			assert_ne!(lhs, rhs);
			Effect::new(move || {
				lhs.gather_subscribers();
				rhs.exec();
			})
		})
		.collect::<Vec<_>>();

	#[thread_local]
	static COUNT: Cell<usize> = Cell::new(0);

	#[cloned(b)]
	let _c = Effect::new(move || {
		a_last.gather_subscribers();
		b.gather_subscribers();
		COUNT.set(COUNT.get() + 1);
	});

	assert_eq!(COUNT.get(), 1);

	drop((a_first.exec(), b.exec()));
	assert_eq!(COUNT.get(), 2);

	drop((b.exec(), a_first.exec()));
	assert_eq!(COUNT.get(), 3);
}
