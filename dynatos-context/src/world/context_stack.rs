//! Context stack

// Imports
use {
	crate::ContextWorld,
	core::{
		any::{Any, TypeId},
		hash::BuildHasher,
		marker::{PhantomData, Unsize},
	},
	dynatos_world::{IMut, IMutLike, WorldGlobal, WorldThreadLocal},
	std::{collections::HashMap, hash::DefaultHasher},
};

/// Context stack
pub trait ContextStack<T: ?Sized, W: ContextWorld>: Sized {
	/// Handle
	type Handle: Copy;

	/// Opaque Handle
	type OpaqueHandle: Copy;

	/// Any type
	type Any: ?Sized + Any + Unsize<dyn Any>;

	/// Pushes a value onto the stack and returns a handle to it
	fn push(value: T) -> Self::Handle
	where
		T: Sized + Unsize<Self::Any> + 'static;

	/// Uses the value in the top of the stack
	fn with_top<F, O>(f: F) -> O
	where
		T: Sized + 'static,
		F: FnOnce(Option<&T>) -> O;

	/// Uses the value in handle `handle`.
	///
	/// # Panics
	/// Panics if the context stack doesn't exist, or
	/// if the value was already taken.
	fn with<F, O>(handle: Self::Handle, f: F) -> O
	where
		T: Sized + 'static,
		F: FnOnce(&T) -> O;

	/// Takes the value in handle `handle`
	fn take(handle: Self::Handle) -> T
	where
		T: Sized + 'static;

	// TODO: Move these next to a separate trait?

	/// Converts a handle to an opaque handle
	fn to_opaque(handle: Self::Handle) -> super::OpaqueHandle<W>;

	/// Uses the value in handle `handle` opaquely.
	///
	/// # Panics
	/// Panics if the context stack doesn't exist, or
	/// if the value was already taken.
	fn with_opaque<F, O>(type_id: TypeId, handle: super::OpaqueHandle<W>, f: F) -> O
	where
		F: FnOnce(&Self::Any) -> O;

	/// Takes the value in handle `handle` opaquely
	fn take_opaque(type_id: TypeId, handle: super::OpaqueHandle<W>) -> Box<Self::Any>;
}

/// Thread-local context stack
pub struct ContextStackThreadLocal<T: ?Sized>(PhantomData<T>);

/// Context stack for `ContextStackThreadLocal`
// TODO: Use type with less indirections?
#[thread_local]
static CTXS_STACK_THREAD_LOCAL: CtxsStackImpl<WorldThreadLocal, dyn Any> =
	IMut::<_, WorldThreadLocal>::new(HashMap::with_hasher(RandomState));

/// Handle for [`ContextStackThreadLocal`]
#[derive(Clone, Copy, Debug)]
pub struct HandleThreadLocal(usize);

impl !Send for HandleThreadLocal {}
impl !Sync for HandleThreadLocal {}

impl<T: ?Sized> ContextStack<T, WorldThreadLocal> for ContextStackThreadLocal<T> {
	type Any = dyn Any;
	type Handle = HandleThreadLocal;
	type OpaqueHandle = HandleThreadLocal;

	fn push(value: T) -> Self::Handle
	where
		T: Sized + Unsize<Self::Any> + 'static,
	{
		let idx = self::push::<WorldThreadLocal, _, _>(&CTXS_STACK_THREAD_LOCAL, value);
		HandleThreadLocal(idx)
	}

	fn with_top<F, O>(f: F) -> O
	where
		T: Sized + 'static,
		F: FnOnce(Option<&T>) -> O,
	{
		self::with_top::<WorldThreadLocal, _, _, _, _>(&CTXS_STACK_THREAD_LOCAL, f)
	}

	fn with<F, O>(handle: Self::Handle, f: F) -> O
	where
		T: Sized + 'static,
		F: FnOnce(&T) -> O,
	{
		self::with::<WorldThreadLocal, _, _, _, _>(&CTXS_STACK_THREAD_LOCAL, handle.0, f)
	}

	fn take(handle: Self::Handle) -> T
	where
		T: Sized + 'static,
	{
		self::take::<WorldThreadLocal, _, _>(&CTXS_STACK_THREAD_LOCAL, handle.0)
	}

	fn to_opaque(handle: Self::Handle) -> super::OpaqueHandle<WorldThreadLocal> {
		handle
	}

	fn with_opaque<F, O>(type_id: TypeId, handle: Self::Handle, f: F) -> O
	where
		F: FnOnce(&Self::Any) -> O,
	{
		self::with_opaque::<WorldThreadLocal, _, _, _>(&CTXS_STACK_THREAD_LOCAL, type_id, handle.0, f)
	}

	fn take_opaque(type_id: TypeId, handle: Self::Handle) -> Box<Self::Any> {
		self::take_opaque::<WorldThreadLocal, _>(&CTXS_STACK_THREAD_LOCAL, type_id, handle.0)
	}
}

/// Global context stack
pub struct ContextStackGlobal<T: ?Sized>(PhantomData<T>);

/// Context stack for `ContextStackGlobal`
// TODO: Use type with less indirections?
static CTXS_STACK_GLOBAL: CtxsStackImpl<WorldGlobal, dyn Any + Send + Sync> =
	IMut::<_, WorldGlobal>::new(HashMap::with_hasher(RandomState));

/// Handle for [`ContextStackGlobal`]
#[derive(Clone, Copy, Debug)]
pub struct HandleGlobal(usize);

impl<T: ?Sized> ContextStack<T, WorldGlobal> for ContextStackGlobal<T> {
	type Any = dyn Any + Send + Sync;
	type Handle = HandleGlobal;
	type OpaqueHandle = HandleGlobal;

	fn push(value: T) -> Self::Handle
	where
		T: Sized + Unsize<Self::Any> + 'static,
	{
		let idx = self::push::<WorldGlobal, _, _>(&CTXS_STACK_GLOBAL, value);
		HandleGlobal(idx)
	}

	fn with_top<F, O>(f: F) -> O
	where
		T: Sized + 'static,
		F: FnOnce(Option<&T>) -> O,
	{
		self::with_top::<WorldGlobal, _, _, _, _>(&CTXS_STACK_GLOBAL, f)
	}

	fn with<F, O>(handle: Self::Handle, f: F) -> O
	where
		T: Sized + 'static,
		F: FnOnce(&T) -> O,
	{
		self::with::<WorldGlobal, _, _, _, _>(&CTXS_STACK_GLOBAL, handle.0, f)
	}

	fn take(handle: Self::Handle) -> T
	where
		T: Sized + 'static,
	{
		self::take::<WorldGlobal, _, _>(&CTXS_STACK_GLOBAL, handle.0)
	}

	fn to_opaque(handle: Self::Handle) -> super::OpaqueHandle<WorldGlobal> {
		handle
	}

	fn with_opaque<F, O>(type_id: TypeId, handle: Self::Handle, f: F) -> O
	where
		F: FnOnce(&Self::Any) -> O,
	{
		self::with_opaque::<WorldGlobal, _, _, _>(&CTXS_STACK_GLOBAL, type_id, handle.0, f)
	}

	fn take_opaque(type_id: TypeId, handle: Self::Handle) -> Box<Self::Any> {
		self::take_opaque::<WorldGlobal, _>(&CTXS_STACK_GLOBAL, type_id, handle.0)
	}
}

type CtxsStackImpl<W, A> = IMut<HashMap<TypeId, CtxStackImpl<A>, RandomState>, W>;
type CtxStackImpl<A> = Vec<Option<Box<A>>>;

/// Hash builder for the stacks
struct RandomState;

impl BuildHasher for RandomState {
	type Hasher = DefaultHasher;

	fn build_hasher(&self) -> Self::Hasher {
		DefaultHasher::default()
	}
}

fn push<W, A, T>(ctxs_stack: &CtxsStackImpl<W, A>, value: T) -> usize
where
	W: ContextWorld,
	A: ?Sized,
	T: Unsize<A> + 'static,
{
	let mut ctxs = ctxs_stack
		.try_write()
		.expect("Cannot modify context while accessing it");
	let stack = ctxs.entry(TypeId::of::<T>()).or_default();
	let idx = stack.len();
	stack.push(Some(Box::new(value) as Box<A>));

	idx
}

fn with_top<W, A, T, F, O>(ctxs_stack: &CtxsStackImpl<W, A>, f: F) -> O
where
	W: ContextWorld,
	A: ?Sized + Any + Unsize<dyn Any>,
	T: 'static,
	F: FnOnce(Option<&T>) -> O,
{
	let type_id = TypeId::of::<T>();
	let ctxs = ctxs_stack.try_read().expect("Cannot access context while modifying it");
	let value = try {
		let stack = ctxs.get(&type_id)?;
		let value = stack.last()?.as_ref().expect("Value was already taken");
		(&**value as &dyn Any)
			.downcast_ref::<T>()
			.expect("Value was the wrong type")
	};

	f(value)
}

fn with<W, A, T, F, O>(ctxs_stack: &CtxsStackImpl<W, A>, idx: usize, f: F) -> O
where
	W: ContextWorld,
	A: ?Sized + Any + Unsize<dyn Any>,
	T: 'static,
	F: FnOnce(&T) -> O,
{
	let type_id = TypeId::of::<T>();
	self::with_opaque::<W, A, _, _>(ctxs_stack, type_id, idx, |value| {
		let value = (value as &dyn Any)
			.downcast_ref::<T>()
			.expect("Value was the wrong type");
		f(value)
	})
}

fn take<W, A, T>(ctxs_stack: &CtxsStackImpl<W, A>, idx: usize) -> T
where
	W: ContextWorld,
	A: ?Sized + Any + Unsize<dyn Any>,
	T: 'static,
{
	let type_id = TypeId::of::<T>();
	let value = self::take_opaque::<W, A>(ctxs_stack, type_id, idx);
	let value = (value as Box<dyn Any>).downcast().expect("Value was the wrong type");

	*value
}


fn with_opaque<W, A, F, O>(ctxs_stack: &CtxsStackImpl<W, A>, type_id: TypeId, idx: usize, f: F) -> O
where
	W: ContextWorld,
	A: ?Sized + 'static,
	F: FnOnce(&A) -> O,
{
	let ctxs = ctxs_stack.try_read().expect("Cannot access context while modifying it");
	let stack = ctxs.get(&type_id).expect("Context stack should exist");
	let value = stack
		.get(idx)
		.expect("Index was invalid")
		.as_ref()
		.expect("Value was already taken");
	f(&**value)
}

fn take_opaque<W, A>(ctxs_stack: &CtxsStackImpl<W, A>, type_id: TypeId, idx: usize) -> Box<A>
where
	W: ContextWorld,
	A: ?Sized,
{
	let mut ctxs = ctxs_stack
		.try_write()
		.expect("Cannot modify context while accessing it");
	let stack = ctxs.get_mut(&type_id).expect("Context stack should exist");
	let value = stack
		.get_mut(idx)
		.and_then(Option::take)
		.expect("Value was already taken");

	// Then remove any empty entries from the end
	while stack.last().is_some_and(|value| value.is_none()) {
		stack.pop().expect("Should have a value at the end");
	}

	value
}
