//! Javascript value

// Imports
use {crate::Object, core::any::Any, std::sync::Arc};

#[derive(Clone, derive_more::Debug)]
enum Inner {
	#[debug("undefined")]
	Undefined,
	#[debug("{_0:?}")]
	String(Arc<str>),
	#[debug("{_0:?}")]
	Bool(bool),
	#[debug("{_0:?}")]
	Float(f64),
	#[debug("{_0:?}")]
	Int(i128),
	#[debug("{_0:?}")]
	Object(Object),

	// TODO: Remove this once we can workaround the bug preventing us from
	//       creating object children in other crates
	#[debug("{_0:p}")]
	Any(Arc<dyn Any + Send + Sync>),
}

#[derive(Clone, derive_more::Debug)]
#[debug("{_0:?}")]
pub struct JsValue(Inner);

impl JsValue {
	pub const UNDEFINED: Self = Self(Inner::Undefined);

	pub fn from_any<T: Any + Send + Sync + 'static>(any: T) -> Self {
		Self(Inner::Any(Arc::new(any)))
	}

	#[must_use]
	pub fn as_any<T: Any + Send + Sync + 'static>(&self) -> Option<&T> {
		match &self.0 {
			Inner::Any(any) => any.downcast_ref(),
			_ => None,
		}
	}

	#[duplicate::duplicate_item(
		fn_name Variant Ty;
		[as_string] [String] [str];
		[as_bool] [Bool] [bool];
		[as_float] [Float] [f64];
		[as_int] [Int] [i128];
		[as_object] [Object] [Object];
	)]
	#[must_use]
	#[expect(clippy::allow_attributes, reason = "Only applicable in some branches")]
	#[allow(clippy::missing_const_for_fn, reason = "Not all branches can be const")]
	pub fn fn_name(&self) -> Option<&Ty> {
		match &self.0 {
			Inner::Variant(value) => Some(value),
			_ => None,
		}
	}

	#[duplicate::duplicate_item(
		fn_name Variant Ty;
		[try_into_string] [String] [Arc<str>];
		[try_into_bool] [Bool] [bool];
		[try_into_float] [Float] [f64];
		[try_into_int] [Int] [i128];
		[try_into_object] [Object] [Object];
	)]
	pub fn fn_name(self) -> Result<Ty, Self> {
		match self.0 {
			Inner::Variant(value) => Ok(value),
			_ => Err(self),
		}
	}
}

#[duplicate::duplicate_item(
	Ty Variant;
	[bool] [Bool];
	[f64] [Float];
)]
impl From<Ty> for JsValue {
	fn from(value: Ty) -> Self {
		Self(Inner::Variant(value))
	}
}

#[duplicate::duplicate_item(
	Ty Variant;
	[&str] [String];
	[String] [String];
	[f32] [Float];
	[i16] [Int];
	[i32] [Int];
	[i64] [Int];
	[i8] [Int];
	[u16] [Int];
	[u32] [Int];
	[u64] [Int];
	[u8] [Int];
)]
impl From<Ty> for JsValue {
	fn from(value: Ty) -> Self {
		Self(Inner::Variant(value.into()))
	}
}

#[duplicate::duplicate_item(
	Ty;
	[isize];
	[usize];
)]
impl From<Ty> for JsValue {
	fn from(value: Ty) -> Self {
		Self(Inner::Int(
			i128::try_from(value).expect(concat!(stringify!(Ty), " did not fit into an `i128`")),
		))
	}
}

#[duplicate::duplicate_item(
	Ty;
	[&String];
)]
impl From<Ty> for JsValue {
	fn from(s: Ty) -> Self {
		Self::from(s.clone())
	}
}

impl From<&Self> for JsValue {
	fn from(value: &Self) -> Self {
		value.clone()
	}
}

impl<T: AsRef<Object>> From<T> for JsValue {
	fn from(value: T) -> Self {
		Self(Inner::Object(value.as_ref().clone()))
	}
}
