//! Reactive element

// Imports
use {
	crate::ObjectAttachEffect,
	dynatos_html::html,
	dynatos_reactive::{Effect, WeakEffect},
	dynatos_util::{TryOrReturnExt, WeakRef},
	std::{
		cell::{Cell, OnceCell, RefCell},
		rc::Rc,
	},
};

/// Creates a reactive element
pub fn dyn_element<F, N>(f: F) -> web_sys::Element
where
	F: Fn() -> Option<N> + 'static,
	N: Into<web_sys::Element>,
{
	// The initial element.
	// This is initialized by the first effect run.
	// Note: This isn't a `OnceCell<WeakRef<Element>>` because the effect doesn't need
	//       to know the initial element, only needs to set it during the first run so
	//       that we get it.
	let init_el = Rc::new(Cell::new(None));

	// The current effect.
	// This is used to re-attach the effect when replacing this element.
	// Note: It's important that this is a `WeakEffect`. Otherwise, we'd get a leak
	//       where the effect owned itself.
	let cur_effect = Rc::new(OnceCell::<WeakEffect>::new());

	// The previous element.
	// When the effect is re-run, this will be updated with the latest element.
	// It is `None` during initialization, but afterwards will always be `Some`.
	// Note: It's important that this is a `WeakRef<Element>`. Otherwise, we'd get a leak
	//       where the element owned the effect, which owned the element.
	let prev_el = RefCell::new(None::<WeakRef<web_sys::Element>>);

	// Setup the effect
	// Note: Throughout this, if `f` returns `None`, we use a `<template>` element to "fake" the
	//       removal of the element. We need to do this to ensure we keep the position on the parent
	//       for when `f` returns `Some` again.
	//       Unlike `with_dyn_child`, we don't cache the empty element, since owning it by value would
	//       create a leak when the current node became it, and owning it by weak ref would destroy it
	//       after `f` returns `Some`.
	let update_effect = Effect::try_new({
		let init_el = init_el.clone();
		let cur_effect = cur_effect.clone();
		move || {
			// Get the current element, or initialize
			let cur_el = match &mut *prev_el.borrow_mut() {
				// If we had one, try to get it.
				// Note: If we can't get it, then the element was dropped, so we'll
				//       be dropped soon as well.
				Some(el) => el.get().or_return()?,

				// Otherwise, we haven't initialized yet, so call `f`, set `prev_el` and `init_el`.
				prev_el @ None => {
					// Note: It's fine to keep `prev_el` borrowed during the call to `f`,
					//       as `f` doesn't have access to anything that could cause a borrow yet.
					let el = f().map(N::into).unwrap_or_else(html::template);

					*prev_el = Some(WeakRef::new(&el));
					init_el.set(Some(el));

					// Note: Important we quit afterwards, since the rest of the function just calls
					//       `f` again and updates the element, which we just did here.
					return;
				},
			};

			// Then get the element to replace it with.
			// Note: If it's the same element as we currently have, we can quit,
			//       since nothing changes.
			let new_el = f().map(N::into).unwrap_or_else(html::template);
			if new_el == cur_el {
				return;
			}

			// If the new element is a sibling of the current element, refuse to update it
			// Note: The typical browser behavior would be to remove the sibling and then add
			//       the new node (or "move" the sibling to our position).
			//       Unfortunately, doing this would mess up other reactive elements by "merging"
			//       their node with ours.
			if let Some(parent) = cur_el.parent_element() {
				if parent.contains(Some(&new_el)) {
					tracing::warn!("Attempted to add the same reactive node multiple times");
					return;
				}
			}

			// Get this effect and attach it to the new element
			// Note: If `get` would return `None`, then this effect was inert.
			//       But if it was inert, we wouldn't be running again, so we
			//       can ensure it exists.
			//       In the same vein, if `upgrade` would return `None`, no more
			//       `Effect`s exist, so we couldn't be running.
			let effect = cur_effect
				.get()
				.expect("Inert reactive element effect was ran again")
				.upgrade()
				.expect("Dropped reactive element effect was ran again");
			new_el.attach_effect(effect);

			// And finally replace the current element with the new one and
			// update the previous element.
			cur_el
				.replace_with_with_node_1(&new_el)
				.expect("Unable to replace element");
			*prev_el.borrow_mut() = Some(WeakRef::new(&new_el));
		}
	});

	// Get the element, then attach the effect, if it wasn't inert
	let el = init_el.take().expect("Should be initialized");
	if let Some(effect) = update_effect {
		cur_effect
			.set(effect.downgrade())
			.expect("Effect initialization was done twice");
		el.attach_effect(effect);
	};

	el
}
