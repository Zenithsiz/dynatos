//! World tags

// Imports
use {super::WORLD, core::cell::Cell};

/// Tag data
#[derive(Clone, Default, Debug)]
struct WorldTagData {
	ref_count: Cell<usize>,
}

/// Guard type for entering and exiting a tag
pub struct WorldTagGuard(WorldTag);

impl Drop for WorldTagGuard {
	fn drop(&mut self) {
		WORLD.tags.remove_tag(self.0);
	}
}

macro decl_tags(
	$WorldTagsData:ident { $get:ident };
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
			$Name,
		)*
	}

	/// Tags data
	#[derive(Clone, Default, Debug)]
	pub struct $WorldTagsData {
		$(
			$field: WorldTagData,
		)*
	}

	impl $WorldTagsData {
		const fn $get(&self, tag: $WorldTag) -> &WorldTagData {
			match tag {
				$(
					$WorldTag::$Name => &self.$field,
				)*
			}
		}
	}
}

decl_tags! {
	WorldTagsData { get };
	WorldTag;

	/// "raw" tag
	Raw(raw),

	/// "unloaded" tag
	Unloaded(unloaded),
}

impl WorldTagsData {
	/// Returns if a tag is present
	pub const fn has_tag(&self, tag: WorldTag) -> bool {
		self.get(tag).ref_count.get() > 0
	}

	/// Adds a tag to the world until the guard is dropped.
	pub fn add_tag(&self, tag: WorldTag) -> WorldTagGuard {
		self.get(tag).ref_count.update(|count| count + 1);
		WorldTagGuard(tag)
	}

	/// Removes a tag from the world
	fn remove_tag(&self, tag: WorldTag) {
		self.get(tag).ref_count.update(|count| count - 1);
	}
}
