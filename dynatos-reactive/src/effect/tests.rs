//! Tests

// Imports
extern crate test;
use {
	super::{super::effect, *},
	core::{
		cell::{Cell, OnceCell},
		mem,
	},
	test::Bencher,
};

/// Ensures effects are executed
#[test]
fn run() {
	#[thread_local]
	static COUNT: Cell<usize> = Cell::new(0);

	assert_eq!(COUNT.get(), 0);
	let effect = Effect::new(|| COUNT.update(|x| x + 1));
	assert_eq!(COUNT.get(), 1);
	effect.run();
	assert_eq!(COUNT.get(), 2);
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

#[bench]
fn get_running_100_none(bencher: &mut Bencher) {
	bencher.iter(|| {
		for _ in 0_usize..100 {
			let effect = effect::running();
			test::black_box(effect);
		}
	});
}

#[bench]
fn get_running_100_some(bencher: &mut Bencher) {
	let effect = Effect::new_raw(move || ());

	effect.gather_dependencies(|| {
		bencher.iter(|| {
			for _ in 0_usize..100 {
				let effect = effect::running();
				test::black_box(effect);
			}
		});
	});
}

#[bench]
fn create_10(bencher: &mut Bencher) {
	bencher.iter(|| {
		for _ in 0_usize..10 {
			let effect = Effect::new(move || ());
			test::black_box(&effect);
			mem::forget(effect);
		}
	});
}
