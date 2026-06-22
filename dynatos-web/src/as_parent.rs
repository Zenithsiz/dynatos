//! Trait to get a type as it's parent

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
		parent: $Parent:ty,
		child_meta: ( $(#[$child_meta:meta])* ),
		child: $Child:ty,
	) => {
		$( #[$child_meta] )*
		impl AsParent<$Parent> for $Child {
			fn as_parent(&self) -> &$Parent {
				self.as_ref()
			}
		}
	},

	// Invokes `@impl_as_parent` for all grandparents
	// and the parent, then recurses on teach grandchild
	// via `@for_each_child`
	(
		@impl_child,
		grandparents: [ $($GrandParent:ty,)* ],
		parent: $Parent:ty,
		child_meta: $child_meta:tt,
		child: $Child:ty: { $(
			$( #[$grandchild_meta:meta] )*
			$GrandChild:ty: $grandchild_inner:tt
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
		parent: $Parent:ty,
		$( child: {
			meta: $child_meta:tt,
			name: $Child:ty,
			inner: $child_inner:tt,
		}, )*
	) => {
		$(
			impl_as_parent! { @impl_child,
				grandparents: $grandparents,
				parent: $Parent,
				child_meta: $child_meta,
				child: $Child: $child_inner,
			}
		)*
	},

	// Main entry point
	(
		$Parent:ty: { $(
			$( #[$child_meta:meta] )*
			$Child:ty: $child_inner:tt
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
	js_sys::Object: {
		web_sys::EventTarget: {
			web_sys::Node: {
				web_sys::Element: {
					web_sys::HtmlElement: {
						web_sys::HtmlBodyElement: {},
						web_sys::HtmlCanvasElement: {},
						web_sys::HtmlDetailsElement: {},
						web_sys::HtmlDialogElement: {},
						web_sys::HtmlHeadElement: {},
						web_sys::HtmlImageElement: {},
						web_sys::HtmlInputElement: {},
						web_sys::HtmlTextAreaElement: {},
					}
				},
				web_sys::Text: {},
				web_sys::Comment: {},
				web_sys::Document: {}
			},
			web_sys::Window: {}
		},
		web_sys::Event: {
			web_sys::AnimationEvent: {},
			web_sys::ClipboardEvent: {},
			web_sys::DragEvent: {},
			web_sys::FocusEvent: {},
			web_sys::InputEvent: {},
			web_sys::MouseEvent: {},
			web_sys::PointerEvent: {},
			web_sys::PopStateEvent: {},
			web_sys::SubmitEvent: {},
			web_sys::ToggleEvent: {},
			web_sys::TransitionEvent: {},
			web_sys::WheelEvent: {},
		},
		web_sys::CssStyleDeclaration: {
			#[cfg(feature = "ssr")]
			web_sys::CssStyleProperties: {},
		},
		web_sys::Location: {},
		web_sys::History: {},
	}
}
