//! `Try` helpers for returning.

// Imports
use core::ops::{ControlFlow, FromResidual, Residual, Try};

/// `Try` type to return `()` when `T::branch` is `Break`.
pub struct TryOrReturn<T>(T);

/// Residual type for [`TryOrReturn`]
pub struct TryOrReturnResidual<T: Try>(T::Residual);

impl<T: Try> Residual<T::Output> for TryOrReturnResidual<T> {
	type TryType = TryOrReturn<T>;
}

impl<T: Try> Try for TryOrReturn<T> {
	type Output = T::Output;
	type Residual = TryOrReturnResidual<T>;

	fn from_output(output: Self::Output) -> Self {
		Self(T::from_output(output))
	}

	fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
		self.0.branch().map_break(TryOrReturnResidual)
	}
}

impl<T: Try> FromResidual<TryOrReturnResidual<T>> for TryOrReturn<T> {
	fn from_residual(residual: TryOrReturnResidual<T>) -> Self {
		Self(T::from_residual(residual.0))
	}
}

impl<T: Try> FromResidual<TryOrReturnResidual<T>> for () {
	fn from_residual(_: TryOrReturnResidual<T>) -> Self {}
}

/// Extension trait to create a [`TryOrReturn`]
#[extend::ext(name = TryOrReturnExt)]
pub impl<T: Try> T {
	fn or_return(self) -> TryOrReturn<T> {
		TryOrReturn(self)
	}
}
