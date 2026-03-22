//! A stack with possible holes

/// A hole-y stack
#[derive(Clone, Debug)]
pub struct HoleyStack<T> {
	values: Vec<Option<T>>,
}

impl<T> HoleyStack<T> {
	/// Creates a new, empty, stack
	#[must_use]
	pub const fn new() -> Self {
		Self { values: vec![] }
	}

	/// Returns if the stack is empty
	#[must_use]
	pub const fn is_empty(&self) -> bool {
		self.values.is_empty()
	}

	/// Pushes a value onto the stack.
	///
	/// Returns the index of the value
	pub fn push(&mut self, value: T) -> usize {
		let idx = self.values.len();
		self.values.push(Some(value));
		idx
	}

	/// Pops an element from the stack by index.
	///
	/// Returns `None` if `idx` is out of bounds or
	/// already taken.
	pub fn pop(&mut self, idx: usize) -> Option<T> {
		let value = self.values.get_mut(idx)?.take()?;
		while self.values.pop_if(|value| value.is_none()).is_some() {}

		Some(value)
	}

	/// Gets a value by index.
	///
	/// Returns `None` if `idx` is out of bounds or
	/// already taken.
	#[must_use]
	pub fn get(&self, idx: usize) -> Option<&T> {
		self.values.get(idx).flatten_ref()
	}

	/// Returns the top of the stack
	#[must_use]
	pub fn top(&self) -> Option<&T> {
		self.values
			.last()
			.map(|last| last.as_ref().expect("Should have a value"))
	}
}

impl<T> Default for HoleyStack<T> {
	fn default() -> Self {
		Self::new()
	}
}
