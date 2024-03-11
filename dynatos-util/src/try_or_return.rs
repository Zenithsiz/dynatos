//! `Try` helpers for returning.

// Imports
use std::ops::{ControlFlow, FromResidual, Try};

/// `Try` type to return `()` when `T::branch` is `Break`.
pub struct TryOrReturn<T>(T);

/// Residual type for [`TryOrReturn`]
pub struct Residual<Res>(Res);

impl<T: Try> Try for TryOrReturn<T> {
	type Output = T::Output;
	type Residual = Residual<T::Residual>;

	fn from_output(output: Self::Output) -> Self {
		Self(T::from_output(output))
	}

	fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
		self.0.branch().map_break(Residual)
	}
}

impl<T: Try> FromResidual<Residual<T::Residual>> for TryOrReturn<T> {
	fn from_residual(residual: Residual<T::Residual>) -> Self {
		Self(T::from_residual(residual.0))
	}
}

impl<Res> FromResidual<Residual<Res>> for () {
	fn from_residual(_: Residual<Res>) -> Self {}
}

/// Extension trait to create a [`TryOrReturn`]
#[extend::ext(name = TryOrReturnExt)]
pub impl<T: Try> T {
	fn or_return(self) -> TryOrReturn<T> {
		TryOrReturn(self)
	}
}
