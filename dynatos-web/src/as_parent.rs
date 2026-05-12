//! Trait to get a type as it's parent

// Imports
use crate::types;

/// Gets this type as it's parent type `T`
pub trait AsParent<T> {
	fn as_parent(&self) -> &T;
}

impl<T> AsParent<T> for T {
	fn as_parent(&self) -> &T {
		self
	}
}

macro impl_as_parent {
	// `AsParent` impl
	(
		@impl_as_parent
		parent: $Parent:ident,
		child_meta: ( $(#[$child_meta:meta])* ),
		child: $Child:ident,
	) => {
		$( #[$child_meta] )*
		impl AsParent<types::$Parent> for types::$Child {
			fn as_parent(&self) -> &types::$Parent {
				self.as_ref()
			}
		}
	},

	// Invokes `@impl_as_parent` for all grandparents
	// and the parent, then recurses on teach grandchild
	// via `@for_each_child`
	(
		@impl_child,
		grandparents: [ $($GrandParent:ident,)* ],
		parent: $Parent:ident,
		child_meta: $child_meta:tt,
		child: $Child:ident { $(
			$( #[$grandchild_meta:meta] )*
			$GrandChild:ident $grandchild_inner:tt
		),* $(,)? },
	) => {
		$(
			impl_as_parent! { @impl_as_parent
				parent: $GrandParent,
				child_meta: $child_meta,
				child: $Child,
			}
		)*

		impl_as_parent! { @impl_as_parent
			parent: $Parent,
			child_meta: $child_meta,
			child: $Child,
		}

		impl_as_parent! { @for_each_child,
			grandparents: [ $($GrandParent,)* $Parent, ],
			parent: $Child,
			$( child: {
				meta: ( $(#[$grandchild_meta])* ),
				name: $GrandChild,
				inner: $grandchild_inner,
			}, )*
		}
	},

	// Separates each child from `children` and invokes
	// `@child_impl` on them.
	(
		@for_each_child,
		grandparents: $grandparents:tt,
		parent: $Parent:ident,
		$( child: {
			meta: $child_meta:tt,
			name: $Child:ident,
			inner: $child_inner:tt,
		}, )*
	) => {
		$(
			impl_as_parent! { @impl_child,
				grandparents: $grandparents,
				parent: $Parent,
				child_meta: $child_meta,
				child: $Child $child_inner,
			}
		)*
	},

	// Main entry point
	(
		$Parent:ident { $(
			$( #[$child_meta:meta] )*
			$Child:ident $child_inner:tt
		),* $(,)? }
	) => {
		impl_as_parent! { @for_each_child,
			grandparents: [],
			parent: $Parent,
			$( child: {
				meta: ( $( #[$child_meta] )* ),
				name: $Child,
				inner: $child_inner,
			}, )*
		}
	}
}

// TODO: Keep this up to date with all the web types we expose.
impl_as_parent! {
	Object {
		EventTarget {
			Node {
				Element {
					HtmlElement {
						HtmlBodyElement {},
						HtmlCanvasElement {},
						HtmlDialogElement {},
						HtmlHeadElement {},
						HtmlImageElement {},
						HtmlInputElement {},
						HtmlTextAreaElement {},
					}
				},
				Text {},
				Comment {},
				Document {}
			},
			Window {}
		},
		Event {
			AnimationEvent {},
			ClipboardEvent {},
			DragEvent {},
			FocusEvent {},
			InputEvent {},
			MouseEvent {},
			PointerEvent {},
			PopStateEvent {},
			SubmitEvent {},
			TransitionEvent {},
			WheelEvent {},
		},
		CssStyleDeclaration {
			#[cfg(feature = "ssr")]
			CssStyleProperties {},
		},
		Location {},
		History {},
	}
}
