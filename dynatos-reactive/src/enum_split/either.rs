//! Either

// Imports
use {
	super::{EnumSplitValue, EnumSplitValueUpdateCtx, SignalStorage},
	crate::{ReactiveWorld, Signal, SignalSet},
};

macro gen_either($Either:ident, $All:ident, $($t:ident: $T:ident),* $(,)?) {
	#[derive(PartialEq, Eq, Clone, Copy, Debug)]
	pub enum $Either< $( $T, )* > {
		$( $T($T), )*
	}

	#[derive(Clone, Copy, Default, Debug)]
	pub struct $All< $( $T, )* > {
		$( $t: $T, )*
	}

	impl<$( $T, )* S, W> EnumSplitValue<S, W> for $Either<$( $T, )*>
	where
		$( $T: Clone + 'static, )*
		S: SignalSet<Self> + Clone + 'static,
		W: ReactiveWorld,
	{
		type SigKind = $Either< $( () ${ignore($T)}, )* >;
		type Signal = $Either< $( Signal<$T>, )* >;
		type SignalsStorage = $All<
			$( Option<SignalStorage<$T>>, )*
		>;

		fn get_signal(storage: &Self::SignalsStorage, cur: &Self::SigKind) -> Option<Self::Signal> {
			let signal = match *cur {
				$(
					$Either::$T(()) => $Either::$T(storage.$t.as_ref()?.signal()),
				)*
			};

			Some(signal)
		}

		fn kind(&self) -> Self::SigKind {
			match *self {
				$(
					Self::$T(_) => $Either::$T(()),
				)*
			}
		}

		fn update(self, storage: &mut Self::SignalsStorage, ctx: EnumSplitValueUpdateCtx<'_, S, W>) {
			match self {
				$(
					Self::$T(new_value) => match &storage.$t {
						Some(storage) => storage.set(new_value),
						None => storage.$t = Some(ctx.create_signal_storage(new_value, Self::$T)),
					},
				)*
			}
		}
	}
}

gen_either! { Either1, All1, t1: T1 }
gen_either! { Either2, All2, t1: T1, t2: T2 }
gen_either! { Either3, All3, t1: T1, t2: T2, t3: T3 }
