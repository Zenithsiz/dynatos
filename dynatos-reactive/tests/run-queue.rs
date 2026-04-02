//! Run queue tests

// Features
#![feature(
	thread_local,
	proc_macro_hygiene,
	stmt_expr_attributes,
	nonpoison_mutex,
	sync_nonpoison
)]

// Imports
use {
	core::iter,
	dynatos_reactive::{Derived, Effect, Signal, SignalBorrowMut, SignalGet, Trigger},
	dynatos_util::Counter,
	std::sync::nonpoison::Mutex,
	zutil_cloned::cloned,
};

/// Ensures that the run queue is breadth-first
#[test]
fn breadth_first() {
	let a = Signal::new(5_usize);
	let b = Signal::new(6_usize);

	static ORDER: Mutex<Vec<&'static str>> = Mutex::new(vec![]);

	#[cloned(a)]
	let a2 = Derived::new(move || {
		ORDER.lock().push("a2");
		a.get() + 1
	});
	#[cloned(b)]
	let b2 = Derived::new(move || {
		ORDER.lock().push("b2");
		b.get() + 1
	});

	let _c = Effect::new(move || {
		ORDER.lock().push("c");
		_ = (a2.get(), b2.get());
	});

	ORDER.lock().clear();

	drop((a.borrow_mut(), b.borrow_mut()));
	assert_eq!(*ORDER.lock(), ["a2", "b2", "c"], "Effect was run with wrong order");
}

#[test]
fn multiple() {
	let a = Trigger::new();

	static COUNT: Counter = Counter::new();
	#[cloned(a)]
	let _effect = Effect::new(move || {
		a.gather_subs();
		a.gather_subs();
		COUNT.bump();
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

	#[expect(clippy::redundant_iter_cloned, reason = "False positive")]
	let _a_effects = a
		.array_windows()
		.cloned()
		.map(move |[lhs, rhs]| {
			assert_ne!(lhs, rhs);
			Effect::new(move || {
				lhs.gather_subs();
				rhs.exec();
			})
		})
		.collect::<Vec<_>>();

	static COUNT: Counter = Counter::new();

	#[cloned(b)]
	let _c = Effect::new(move || {
		a_last.gather_subs();
		b.gather_subs();
		COUNT.bump();
	});

	assert_eq!(COUNT.get(), 1);

	drop((a_first.exec(), b.exec()));
	assert_eq!(COUNT.get(), 2);

	drop((b.exec(), a_first.exec()));
	assert_eq!(COUNT.get(), 3);
}
