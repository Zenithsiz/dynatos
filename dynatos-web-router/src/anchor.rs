//! Anchor element

// Imports
use {
	dynatos_sync_types::SyncBounds,
	dynatos_web::{
		DynatosWebCtx,
		ElementWithAttr,
		html,
		types::{HtmlElement, cfg_ssr_expr},
	},
};

/// Creates a reactive anchor element.
///
/// Expects a value of type [`LocationSignal`](crate::LocationSignal) in the context store.
pub fn anchor<U>(ctx: &DynatosWebCtx, new_location: U) -> HtmlElement
where
	U: SyncBounds + AsRef<str> + 'static,
{
	let link = html::a(ctx).with_attr("href", new_location.as_ref());

	cfg_ssr_expr!(
		ssr = {
			let _: &DynatosWebCtx = ctx;

			link
		},
		csr = {
			use {
				crate::LocationSignal,
				dynatos_reactive::{SignalBorrow, SignalSet},
				dynatos_web::{EventTargetWithListener, ev},
			};

			let location = ctx.store().get::<LocationSignal>();

			link.with_event_listener::<ev!(click)>(ctx, move |ev| {
				ev.prevent_default();

				let new_location = new_location.as_ref();
				let res = location.borrow_no_dep().join(new_location);
				match res {
					Ok(new_location) => location.set(new_location),
					Err(err) => tracing::warn!("Unable to join new location to current {new_location:?}: {err}"),
				}
			})
		}
	)
}
