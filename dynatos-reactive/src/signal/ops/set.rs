//! [`SignalSet`]

// Imports
use crate::{effect, SignalUpdate, Trigger};

/// Types which may be set by [`SignalSet`]
pub trait SignalSetWith<T>: Sized {
	fn set_value(self, new_value: T);
}

impl<T> SignalSetWith<T> for &'_ mut T {
	fn set_value(self, new_value: T) {
		*self = new_value;
	}
}
impl<T> SignalSetWith<T> for &'_ mut Option<T> {
	fn set_value(self, new_value: T) {
		*self = Some(new_value);
	}
}

/// Auto trait implemented for all signals that want a default implementation of `SignalSet`
///
/// If you are writing a signal type with type parameters, you should manually implement
/// this auto trait, since those type parameters might disable it
pub auto trait SignalSetDefaultImpl {}

/// Signal set
pub trait SignalSet<Value> {
	/// Sets the signal value
	#[track_caller]
	fn set(&self, new_value: Value);

	/// Sets the signal value without updating dependencies
	#[track_caller]
	fn set_raw(&self, new_value: Value) {
		effect::with_raw(|| self.set(new_value));
	}
}

impl<S, T> SignalSet<T> for S
where
	S: for<'a> SignalUpdate<Value<'a>: SignalSetWith<T>> + SignalSetDefaultImpl,
{
	fn set(&self, new_value: T) {
		self.update(|value| SignalSetWith::set_value(value, new_value));
	}
}

macro impl_tuple($($S:ident : $T:ident),* $(,)?) {
	#[allow(clippy::allow_attributes, non_snake_case, reason = "Macro generated code")]
	impl<$( $S, $T, )*> SignalSet<( $( $T, )* )> for ( $( &'_ $S, )* )
	where
		$( $S: SignalSet<$T>, )*
	{
		fn set(&self, new_value: ( $( $T, )* )) {
			// Note: We use a no-op exec to ensure that we only run the queue once
			//       during both of the sets.
			let _exec = Trigger::exec_noop();

			let ( $( $S, )* ) = self;
			let ( $( $T, )* ) = new_value;
			$( $S.set($T); )*
		}
	}
}

impl_tuple! {}
impl_tuple! { S1: T1, }
impl_tuple! { S1: T1, S2: T2, }
impl_tuple! { S1: T1, S2: T2, S3: T3 }
impl_tuple! { S1: T1, S2: T2, S3: T3, S4: T4 }
impl_tuple! { S1: T1, S2: T2, S3: T3, S4: T4, S5: T5 }
