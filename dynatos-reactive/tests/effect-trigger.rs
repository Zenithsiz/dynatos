//! Effect-trigger tests

// Features
#![feature(thread_local, proc_macro_hygiene, stmt_expr_attributes)]

// Imports
use {
	core::{
		cell::{Cell, OnceCell},
		mem,
	},
	dynatos_reactive::{effect, Effect, Trigger, WeakEffect, WeakTrigger},
	zutil_cloned::cloned,
};

#[test]
fn basic() {
	/// Counts the number of times the effect was run
	#[thread_local]
	static TRIGGERS: Cell<usize> = Cell::new(0);

	let trigger = Trigger::new();
	#[cloned(trigger)]
	let effect = Effect::new(move || {
		trigger.gather_subs();
		TRIGGERS.set(TRIGGERS.get() + 1);
	});

	assert_eq!(TRIGGERS.get(), 1, "Trigger was triggered early");

	// Then trigger and ensure it was triggered
	trigger.exec();
	assert_eq!(TRIGGERS.get(), 2, "Trigger was not triggered");

	// Finally drop the effect and try again
	mem::drop(effect);
	trigger.exec();
	assert_eq!(TRIGGERS.get(), 2, "Trigger was triggered after effect was dropped");
}

#[test]
fn trigger_exec_multiple() {
	/// Counts the number of times the effect was run
	#[thread_local]
	static TRIGGERS: Cell<usize> = Cell::new(0);

	let trigger = Trigger::new();
	#[cloned(trigger)]
	let _effect = Effect::new(move || {
		trigger.gather_subs();
		TRIGGERS.set(TRIGGERS.get() + 1);
	});

	let exec0 = trigger.exec();
	assert_eq!(TRIGGERS.get(), 1, "Trigger was triggered when executing");
	let exec1 = trigger.exec();

	drop(exec1);
	assert_eq!(
		TRIGGERS.get(),
		1,
		"Trigger was triggered when dropping a single executor"
	);

	drop(exec0);
	assert_eq!(
		TRIGGERS.get(),
		2,
		"Trigger wasn't triggered when dropping last executor"
	);
}

#[test]
fn exec_multiple_same_effect() {
	/// Counts the number of times the effect was run
	#[thread_local]
	static TRIGGERS: Cell<usize> = Cell::new(0);

	let trigger0 = Trigger::new();
	let trigger1 = Trigger::new();
	#[cloned(trigger0, trigger1)]
	let _effect = Effect::new(move || {
		trigger0.gather_subs();
		trigger1.gather_subs();
		TRIGGERS.set(TRIGGERS.get() + 1);
	});

	let exec0 = trigger0.exec();
	let exec1 = trigger1.exec();

	drop((exec0, exec1));

	assert_eq!(TRIGGERS.get(), 2, "Effect was run multiple times in same run queue");

	trigger0.exec();

	assert_eq!(
		TRIGGERS.get(),
		3,
		"Effect wasn't run even when no other executors existed"
	);
}

/// Ensures effects are executed only when stale
#[test]
fn fresh_stale() {
	#[thread_local]
	static COUNT: Cell<usize> = Cell::new(0);

	assert_eq!(COUNT.get(), 0);
	let effect = Effect::new(|| COUNT.update(|x| x + 1));
	assert_eq!(COUNT.get(), 1, "Effect wasn't run on creation");
	effect.run();
	assert_eq!(COUNT.get(), 1, "Effect was ran despite being fresh");
	effect.set_stale();
	effect.run();
	assert_eq!(COUNT.get(), 2, "Effect wasn't run despite being stale");
	effect.run();
	assert_eq!(COUNT.get(), 2, "Effect was ran despite being fresh");

	effect.force_run();
	assert_eq!(COUNT.get(), 3, "Effect wasn't run when fresh despite force running");
}

/// Ensures the function returned by `Effect::running` is the same as the future being run.
#[test]
fn running() {
	#[thread_local]
	static RUNNING: OnceCell<Effect> = OnceCell::new();

	// Create an effect, and save the running effect within it to `RUNNING`.
	let effect = Effect::new(move || {
		RUNNING
			.set(effect::running().expect("Future wasn't running"))
			.expect("Unable to set running effect");
	});

	// Then ensure the running effect is the same as the one created.
	let running = RUNNING.get().expect("Running effect missing");
	assert_eq!(effect, *running);
}

/// Ensures the function returned by `Effect::running` is the same as the future being run,
/// while running stacked futures
#[test]
fn running_stacked() {
	#[thread_local]
	static RUNNING_TOP: OnceCell<Effect> = OnceCell::new();

	#[thread_local]
	static RUNNING_BOTTOM: OnceCell<Effect> = OnceCell::new();

	// Create 2 stacked effects, saving the running within each to `running1` and `running2`.
	// `running1` contains the top-level effect, while `running2` contains the inner one.
	let effect = Effect::new(move || {
		RUNNING_TOP
			.set(effect::running().expect("Future wasn't running"))
			.expect("Unable to set running effect");

		let effect = Effect::new(move || {
			RUNNING_BOTTOM
				.set(effect::running().expect("Future wasn't running"))
				.expect("Unable to set running effect");
		});

		// Then ensure the bottom-level running effect is the same as the one created.
		let running_bottom = RUNNING_BOTTOM.get().expect("Running effect missing");
		assert_eq!(effect, *running_bottom);
	});

	// Then ensure the top-level running effect is the same as the one created.
	let running_top = RUNNING_TOP.get().expect("Running effect missing");
	assert_eq!(effect, *running_top);

	// And that the bottom-level running effect is already inert
	let running_bottom = RUNNING_BOTTOM.get().expect("Running effect missing");
	assert!(running_bottom.is_inert());
}

#[test]
fn weak_effect_empty() {
	let effect = WeakEffect::<fn()>::new();
	assert_eq!(effect.upgrade(), None);
	assert!(!effect.try_run());

	let unsize = effect.unsize();
	assert_eq!(effect, unsize);
}

#[test]
fn weak_effect_dropped() {
	let effect = Effect::new(|| {}).downgrade();
	assert_eq!(effect.upgrade(), None);
	assert!(!effect.try_run());

	let unsize = effect.unsize();
	// TODO: The two should be equal, not different.
	assert_ne!(effect, unsize);
}

#[test]
fn weak_trigger_empty() {
	let trigger = WeakTrigger::new();
	assert_eq!(trigger.upgrade(), None);
}

#[test]
fn trigger_upgrade() {
	let trigger = Trigger::new();
	let weak = trigger.downgrade();

	assert_eq!(Some(trigger), weak.upgrade());
}
