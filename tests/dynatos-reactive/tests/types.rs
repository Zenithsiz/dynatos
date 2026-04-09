//! Type tests

// Features
#![feature(decl_macro)]
#![cfg(feature = "sync")]

// Imports
use {
	core::future,
	dynatos_reactive::{
		AsyncSignal,
		Derived,
		Effect,
		EnumSplitSignal,
		GlobalWorld,
		MappedSignal,
		Memo,
		Signal,
		Trigger,
		TryMappedSignal,
		WeakTrigger,
		WithDefault,
	},
};

const fn is_send_sync<T: Send + Sync>() {}

macro are_send_sync($($T:ty),* $(,)?) {
	const _: () = {
		$( self::is_send_sync::<$T>(); )*
	};
}

are_send_sync! {
	AsyncSignal<fn() -> future::Ready<i32>>,
	Derived<i32, fn() -> i32>,
	Effect,
	EnumSplitSignal<Signal<Option<i32>>, Option<i32>>,
	TryMappedSignal<Result<i32, i32>>,
	MappedSignal<i32>,
	Memo<i32, fn() -> i32>,
	Signal<i32>,
	Trigger,
	WeakTrigger,
	WithDefault<Signal<i32>, i32>,
	GlobalWorld,
}
