//! Location

// Imports
use {
	core::ops::{Deref, DerefMut},
	dynatos_reactive::{Signal, SignalBorrow, SignalBorrowMut, signal},
	dynatos_web::{DynatosWebCtx, EventTargetAddListener, ev},
	dynatos_web::types::JsValue,
	url::Url,
	zutil_cloned::cloned,
};

/// Inner
#[derive(Debug)]
struct Inner {
	/// Location
	location: Url,

	ctx: DynatosWebCtx,
}

/// Location
// TODO: Rename to avoid confusing with web's `Location`?
#[derive(Clone, Debug)]
pub struct Location(Signal<Inner>);

impl Location {
	/// Creates a new location
	#[must_use]
	#[track_caller]
	pub fn new(ctx: &DynatosWebCtx) -> Self {
		let location = self::parse_location_url(ctx);
		let inner = Inner {
			location,
			ctx: ctx.clone(),
		};
		let inner = Signal::new(inner);

		// Add an event listener on the document for when the user navigates manually
		#[cloned(ctx, inner)]
		let update = move |_ev| {
			let new_location = self::parse_location_url(&ctx);
			inner.borrow_mut().location = new_location;
		};
		ctx.window().add_event_listener::<ev!(popstate)>(ctx, update);

		Self(inner)
	}
}

/// Reference type for [`SignalBorrow`] impl
#[derive(Debug)]
pub struct BorrowRef<'a>(signal::BorrowRef<'a, Inner>);

impl Deref for BorrowRef<'_> {
	type Target = Url;

	fn deref(&self) -> &Self::Target {
		&self.0.location
	}
}

impl SignalBorrow for Location {
	type Ref<'a>
		= BorrowRef<'a>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		BorrowRef(self.0.borrow())
	}
}

/// Reference type for [`SignalBorrowMut`] impl
#[derive(Debug)]
pub struct BorrowRefMut<'a>(Option<signal::BorrowRefMut<'a, Inner>>);

impl Deref for BorrowRefMut<'_> {
	type Target = Url;

	fn deref(&self) -> &Self::Target {
		&self.0.as_ref().expect("Should exist").location
	}
}

impl DerefMut for BorrowRefMut<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0.as_mut().expect("Should exist").location
	}
}

impl Drop for BorrowRefMut<'_> {
	fn drop(&mut self) {
		// Note: We need to drop the borrow *before* pushing the url because with
		//       SSR, changing the url immediately calls any events, which can try
		//       to borrow us and deadlock.
		let borrow = self.0.take().expect("Should exist");
		let history = borrow.ctx.history().clone();
		let location = borrow.location.clone();
		let _exec_guard = borrow.into_trigger_exec();

		// Push the new location into history
		match history.push_state_with_url(&JsValue::UNDEFINED, "", Some(location.as_str())) {
			Ok(()) => tracing::debug!("Pushed history: {:?}", location.as_str()),
			Err(err) => tracing::error!("Unable to push history {:?}: {err:?}", location.as_str()),
		}
	}
}

impl SignalBorrowMut for Location {
	type RefMut<'a>
		= BorrowRefMut<'a>
	where
		Self: 'a;

	fn borrow_mut(&self) -> Self::RefMut<'_> {
		let value = self.0.borrow_mut();
		BorrowRefMut(Some(value))
	}
}

/// Parses the location as url
fn parse_location_url(ctx: &DynatosWebCtx) -> Url {
	let location = ctx.location().href().expect("Unable to get location href");
	location.parse::<Url>().expect("Location href was an invalid url")
}
