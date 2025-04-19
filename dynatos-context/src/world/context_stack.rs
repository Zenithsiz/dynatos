//! Context stack

// Imports
use {
	crate::ContextWorld,
	core::{
		any::{Any, TypeId},
		hash::BuildHasher,
		marker::Unsize,
	},
	dynatos_world::{IMut, IMutLike, WorldGlobal, WorldThreadLocal},
	std::{collections::HashMap, hash::DefaultHasher},
};

/// Context stack
pub trait ContextStack<W: ContextWorld>: Sized {
	/// Any type
	type Any: ?Sized + Any + Unsize<dyn Any>;

	/// Handle bounds
	type HandleBounds;

	/// Boxes a value int the any type
	fn box_any<T>(value: T) -> Box<Self::Any>
	where
		T: Unsize<Self::Any>;

	/// Uses the context stack for `T`
	fn with<T, F, O>(f: F) -> O
	where
		T: 'static,
		F: FnOnce(Option<&CtxStack<Self::Any>>) -> O,
	{
		let type_id = TypeId::of::<T>();
		Self::with_opaque(type_id, f)
	}

	/// Uses the context stack for `T` with a type id
	fn with_opaque<F, O>(type_id: TypeId, f: F) -> O
	where
		F: FnOnce(Option<&CtxStack<Self::Any>>) -> O;

	/// Uses the context stack for `T` mutably
	fn with_mut<T, F, O>(f: F) -> O
	where
		T: 'static,
		F: FnOnce(&mut CtxStack<Self::Any>) -> O,
	{
		let type_id = TypeId::of::<T>();
		Self::with_mut_opaque(type_id, f)
	}

	/// Uses the context stack for `T` mutably with a type id
	fn with_mut_opaque<F, O>(type_id: TypeId, f: F) -> O
	where
		F: FnOnce(&mut CtxStack<Self::Any>) -> O;
}

/// Thread-local context stack
pub struct ContextStackThreadLocal;

/// Context stack for `ContextStackThreadLocal`
// TODO: Use type with less indirections?
#[thread_local]
static CTXS_STACK_THREAD_LOCAL: CtxsStack<WorldThreadLocal, dyn Any> =
	IMut::<_, WorldThreadLocal>::new(HashMap::with_hasher(RandomState));

impl ContextStack<WorldThreadLocal> for ContextStackThreadLocal {
	type Any = dyn Any;
	type HandleBounds = *mut u8;

	fn box_any<T>(value: T) -> Box<Self::Any>
	where
		T: Unsize<Self::Any>,
	{
		Box::new(value)
	}

	fn with_opaque<F, O>(type_id: TypeId, f: F) -> O
	where
		F: FnOnce(Option<&CtxStack<Self::Any>>) -> O,
	{
		let ctxs = CTXS_STACK_THREAD_LOCAL
			.try_read()
			.expect("Cannot access context while modifying it");
		let stack = ctxs.get(&type_id);
		f(stack)
	}

	fn with_mut_opaque<F, O>(type_id: TypeId, f: F) -> O
	where
		F: FnOnce(&mut CtxStack<Self::Any>) -> O,
	{
		let mut ctxs = CTXS_STACK_THREAD_LOCAL
			.try_write()
			.expect("Cannot modify context while accessing it");
		let stack = ctxs.entry(type_id).or_default();
		f(stack)
	}
}

/// Global context stack
pub struct ContextStackGlobal;

/// Context stack for `ContextStackGlobal`
// TODO: Use type with less indirections?
static CTXS_STACK_GLOBAL: CtxsStack<WorldGlobal, dyn Any + Send + Sync> =
	IMut::<_, WorldGlobal>::new(HashMap::with_hasher(RandomState));

#[expect(clippy::significant_drop_tightening, reason = "False positive")]
impl ContextStack<WorldGlobal> for ContextStackGlobal {
	type Any = dyn Any + Send + Sync;
	type HandleBounds = ();

	fn box_any<T>(value: T) -> Box<Self::Any>
	where
		T: Unsize<Self::Any>,
	{
		Box::new(value)
	}

	fn with_opaque<F, O>(type_id: TypeId, f: F) -> O
	where
		F: FnOnce(Option<&CtxStack<Self::Any>>) -> O,
	{
		let ctxs = CTXS_STACK_GLOBAL
			.try_read()
			.expect("Cannot access context while modifying it");
		let stack = ctxs.get(&type_id);
		f(stack)
	}

	fn with_mut_opaque<F, O>(type_id: TypeId, f: F) -> O
	where
		F: FnOnce(&mut CtxStack<Self::Any>) -> O,
	{
		let mut ctxs = CTXS_STACK_GLOBAL
			.try_write()
			.expect("Cannot modify context while accessing it");
		let stack = ctxs.entry(type_id).or_default();
		f(stack)
	}
}

type CtxsStack<W, A> = IMut<HashMap<TypeId, CtxStack<A>, RandomState>, W>;
type CtxStack<A> = Vec<Option<Box<A>>>;

/// Hash builder for the stacks
struct RandomState;

impl BuildHasher for RandomState {
	type Hasher = DefaultHasher;

	fn build_hasher(&self) -> Self::Hasher {
		DefaultHasher::default()
	}
}
