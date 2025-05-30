//! Title management for `dynatos`

// TODO: It seems that titles aren't getting dropped for some reason.

// Features
#![feature(thread_local)]

// Imports
use {core::cell::RefCell, dynatos_html::ObjectSetProp, wasm_bindgen::prelude::wasm_bindgen};

/// Title stack.
#[thread_local]
static TITLE_STACK: RefCell<Vec<Option<String>>> = RefCell::new(vec![]);

/// Title.
///
/// Sets the title for as long as this lives.
#[derive(Debug)]
pub struct Title {
	/// Title index
	title_idx: usize,
}

impl Title {
	/// Creates a title.
	pub fn new<S>(title: S) -> Self
	where
		S: Into<String>,
	{
		let title: String = title.into();

		// If no title exists, add the current one
		let mut stack = TITLE_STACK.borrow_mut();
		if stack.is_empty() {
			stack.push(Some(self::cur_title()));
		}

		// Then set the title and add ours to the stack
		self::set_title(&title);
		let title_idx = stack.len();
		stack.push(Some(title));

		Self { title_idx }
	}
}

impl Drop for Title {
	fn drop(&mut self) {
		// Remove our title
		let mut stack = TITLE_STACK.borrow_mut();
		let _prev_title = stack
			.get_mut(self.title_idx)
			.and_then(Option::take)
			.expect("Title was already taken");

		// Then find the next title to set back to.
		let next_title = loop {
			let last = stack.last().expect("Should contain at least 1 title");
			match last {
				Some(title) => break title,
				None => {
					stack.pop().expect("Just checked the value existed");
				},
			}
		};
		self::set_title(next_title);
	}
}

/// Extension trait to attach a title to an object.
#[extend::ext(name = ObjectAttachTitle)]
pub impl js_sys::Object {
	/// Attaches a title to this object
	fn attach_title(&self, title: &str) {
		let prop_name = "__dynatos_title";
		let title = Title::new(title);
		self.set_prop(prop_name, WasmTitle(title));
	}
}

/// Extension trait to attach a title to an object.
#[extend::ext(name = ObjectWithTitle)]
pub impl<T> T
where
	T: AsRef<js_sys::Object>,
{
	/// Attaches a title to this object.
	///
	/// Returns the object, for chaining
	fn with_title(self, title: &str) -> Self {
		self.as_ref().attach_title(title);
		self
	}
}

/// Returns the current title
fn cur_title() -> String {
	web_sys::window()
		.expect("Unable to get window")
		.document()
		.expect("Unable to get document")
		.title()
}

/// Sets the title
fn set_title(title: &str) {
	web_sys::window()
		.expect("Unable to get window")
		.document()
		.expect("Unable to get document")
		.set_title(title);
}

/// A wasm `Title` type.
#[wasm_bindgen]
#[expect(dead_code, reason = "We just want to keep the field alive, not use it")]
struct WasmTitle(Title);
