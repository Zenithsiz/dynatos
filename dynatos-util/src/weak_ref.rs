//! Javascript weak references

// Imports
use {std::marker::PhantomData, wasm_bindgen::JsCast};

/// Javascript weak reference
pub struct WeakRef<T> {
	/// Inner value
	inner: sys::WeakRef,

	/// Phantom
	// TODO: Is the variance correct?
	_phantom: PhantomData<T>,
}

impl<T> WeakRef<T> {
	/// Creates a new weak reference
	pub fn new(value: &T) -> Self
	where
		T: AsRef<js_sys::Object>,
	{
		let inner = sys::WeakRef::new(value.as_ref());
		Self {
			inner,
			_phantom: PhantomData,
		}
	}

	/// Returns the inner value
	pub fn get(&self) -> Option<T>
	where
		T: JsCast,
	{
		self.inner
			.deref()
			.map(|value| value.dyn_into::<T>().expect("Inner value was of the wrong type"))
	}
}


/// System bindings for `WeakRef`
pub mod sys {
	use wasm_bindgen::prelude::wasm_bindgen;

	#[wasm_bindgen]
	extern "C" {
		/// Weak reference.
		///
		/// [MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/WeakRef)
		#[wasm_bindgen(js_name = WeakRef)]
		pub type WeakRef;

		/// Constructor
		///
		/// [MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/WeakRef/WeakRef)
		#[wasm_bindgen(constructor)]
		pub fn new(target: &js_sys::Object) -> WeakRef;

		/// Dereference method
		///
		/// [MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/WeakRef/deref)
		#[wasm_bindgen(method, js_name = deref)]
		pub fn deref(this: &WeakRef) -> Option<js_sys::Object>;
	}
}
