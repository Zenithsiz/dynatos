//! Anchor element

// Imports
use {
	crate::Location,
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
/// Expects a context of type [`Location`](crate::Location).
#[cfg_attr(
	feature = "ssr",
	expect(clippy::needless_pass_by_value, reason = "Necessary for `csr`")
)]
pub fn anchor<U>(ctx: &DynatosWebCtx, location: Location, new_location: U) -> HtmlElement
where
	U: SyncBounds + AsRef<str> + 'static,
{
	let link = html::a(ctx).with_attr("href", new_location.as_ref());

	cfg_ssr_expr!(
		ssr = {
			let _: &DynatosWebCtx = ctx;
			let _: Location = location;

			link
		},
		csr = {
			use {
				dynatos_reactive::{SignalBorrow, SignalSet},
				dynatos_web::{EventTargetWithListener, ev},
			};

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
