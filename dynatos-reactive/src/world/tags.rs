//! World tags

// Imports
use {super::WORLD, core::cell::RefCell};

/// Tag state
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum WorldTagState {
	Enabled,
	Disabled,
}

/// Tag data
#[derive(Default, Debug)]
struct WorldTagData {
	stack: RefCell<Vec<Option<WorldTagState>>>,
}

/// Guard type for entering and exiting a tag
pub struct WorldTagGuard {
	tag: WorldTag,
	idx: usize,
}

impl Drop for WorldTagGuard {
	fn drop(&mut self) {
		WORLD.tags.pop(self.tag, self.idx);
	}
}

macro decl_tags(
	$WorldTagsData:ident { $get_data:ident };
	$WorldTag:ident;

	$(
		$( #[$meta:meta] )*
		$Name:ident($field:ident)
	),* $(,)?
) {
	/// Tags
	#[derive(PartialEq, Eq, Clone, Copy, Debug)]
	pub enum $WorldTag {
		$(
			$( #[$meta] )*
			$Name,
		)*
	}

	/// Tags data
	#[derive(Default, Debug)]
	pub struct $WorldTagsData {
		$(
			$field: WorldTagData,
		)*
	}

	impl $WorldTagsData {
		const fn $get_data(&self, tag: $WorldTag) -> &WorldTagData {
			match tag {
				$(
					$WorldTag::$Name => &self.$field,
				)*
			}
		}
	}
}

decl_tags! {
	WorldTagsData { get_data };
	WorldTag;

	/// "raw" tag.
	///
	/// This tag is cleared when running effects, and so
	/// only affects the current reactivity frame.
	Raw(raw),

	/// "unloaded" tag.
	///
	/// This tag is never removed automatically, to ensure
	/// that any effects being run also don't load anything.
	Unloaded(unloaded),
}

impl WorldTagsData {
	/// Returns a tag's state, if any
	pub fn get(&self, tag: WorldTag) -> Option<WorldTagState> {
		self.get_data(tag)
			.stack
			.borrow()
			.last()
			.map(|enabled| enabled.expect("Last tag on the stack was dropped"))
	}

	/// Pushes a tag onto the stack
	pub fn push(&self, tag: WorldTag, state: WorldTagState) -> WorldTagGuard {
		let tag_data = self.get_data(tag);
		let mut stack = tag_data.stack.borrow_mut();

		let idx = stack.len();
		stack.push(Some(state));

		WorldTagGuard { tag, idx }
	}

	/// Pops a tag from the world
	fn pop(&self, tag: WorldTag, idx: usize) {
		let tag_data = self.get_data(tag);
		let mut stack = tag_data.stack.borrow_mut();

		stack[idx] = None;
		while stack.pop_if(|tag| tag.is_none()).is_some() {}
	}
}
