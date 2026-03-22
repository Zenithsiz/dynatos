//! Context stack

// Lints
#![expect(clippy::as_conversions, reason = "We need to unsize items and there's no other way")]

// Imports
use {
	core::{
		any::{Any, TypeId},
		cell::RefCell,
		hash::BuildHasherDefault,
		marker::PhantomData,
	},
	dynatos_util::HoleyStack,
	std::{collections::HashMap, hash::DefaultHasher},
};

/// Context stack
// TODO: Use type with less indirections?
#[thread_local]
static CTXS_STACK: CtxsStackImpl<dyn Any> = RefCell::new(HashMap::with_hasher(BuildHasherDefault::new()));

/// Handle
#[derive(Debug)]
pub struct Handle<T>(usize, PhantomData<T>);

impl<T> Clone for Handle<T> {
	fn clone(&self) -> Self {
		*self
	}
}
impl<T> Copy for Handle<T> {}

impl<T> !Send for Handle<T> {}
impl<T> !Sync for Handle<T> {}

/// Opaque handle
#[derive(Clone, Copy, Debug)]
pub struct OpaqueHandle {
	type_id: TypeId,
	idx:     usize,
}

impl !Send for OpaqueHandle {}
impl !Sync for OpaqueHandle {}

/// Pushes a value onto the stack and returns a handle to it
pub fn push<T>(value: T) -> Handle<T>
where
	T: Any + 'static,
{
	let mut ctxs = CTXS_STACK
		.try_borrow_mut()
		.expect("Cannot modify context while accessing it");
	let stack = ctxs.entry(TypeId::of::<T>()).or_default();
	let idx = stack.push(Box::new(value));

	Handle(idx, PhantomData)
}

/// Uses the value in the top of the stack
pub fn with_top<T, F, O>(f: F) -> O
where
	T: 'static,
	F: FnOnce(Option<&T>) -> O,
{
	let type_id = TypeId::of::<T>();
	let ctxs = CTXS_STACK
		.try_borrow()
		.expect("Cannot access context while modifying it");

	let value = try {
		let stack = ctxs.get(&type_id)?;
		let value = stack.top()?;
		(&**value as &dyn Any)
			.downcast_ref::<T>()
			.expect("Value was the wrong type")
	};

	f(value)
}

/// Uses the value in handle `handle`.
///
/// # Panics
/// Panics if the context stack doesn't exist, or
/// if the value was already taken.
pub fn with<T, F, O>(handle: Handle<T>, f: F) -> O
where
	T: 'static,
	F: FnOnce(&T) -> O,
{
	let opaque_handle = self::to_opaque::<T>(handle);
	self::with_opaque(opaque_handle, |value| {
		let value = (value as &dyn Any)
			.downcast_ref::<T>()
			.expect("Value was the wrong type");
		f(value)
	})
}

/// Takes the value in handle `handle`
#[expect(clippy::must_use_candidate, reason = "The user may just want to pop the value")]
pub fn take<T>(handle: Handle<T>) -> T
where
	T: 'static,
{
	let opaque_handle = self::to_opaque::<T>(handle);
	let value = self::take_opaque(opaque_handle);
	let value = (value as Box<dyn Any>).downcast().expect("Value was the wrong type");

	*value
}

/// Converts a handle to an opaque handle
#[must_use]
pub const fn to_opaque<T>(handle: Handle<T>) -> OpaqueHandle
where
	T: 'static,
{
	OpaqueHandle {
		type_id: TypeId::of::<T>(),
		idx:     handle.0,
	}
}

/// Uses the value in handle `handle` opaquely.
///
/// # Panics
/// Panics if the context stack doesn't exist, or
/// if the value was already taken.
pub fn with_opaque<F, O>(handle: OpaqueHandle, f: F) -> O
where
	F: FnOnce(&dyn Any) -> O,
{
	let ctxs = CTXS_STACK
		.try_borrow()
		.expect("Cannot access context while modifying it");
	let stack = ctxs.get(&handle.type_id).expect("Context stack should exist");
	let value = stack.get(handle.idx).expect("Value was already taken");
	f(&**value)
}

/// Takes the value in handle `handle` opaquely
pub fn take_opaque(handle: OpaqueHandle) -> Box<dyn Any> {
	let mut ctxs = CTXS_STACK
		.try_borrow_mut()
		.expect("Cannot modify context while accessing it");
	let stack = ctxs.get_mut(&handle.type_id).expect("Context stack should exist");

	stack.pop(handle.idx).expect("Value was already taken")
}

type CtxsStackImpl<A> = RefCell<HashMap<TypeId, CtxStackImpl<A>, BuildHasherDefault<DefaultHasher>>>;
type CtxStackImpl<A> = HoleyStack<Box<A>>;
