//! Query example

// Features
#![feature(stmt_expr_attributes, proc_macro_hygiene)]

// Imports
use {
	app_error::AppError,
	dynatos_loadable::Loadable,
	dynatos_reactive::{SignalBorrowMut, SignalGetCloned, SignalSet},
	dynatos_web::{DynatosWebCtx, EventTargetWithListener, NodeWithChildren, NodeWithText, ev, html},
	dynatos_web_reactive::NodeWithDynText,
	dynatos_web_router::{Location, QuerySignal, SingleQuery},
	tracing_subscriber::prelude::*,
	zutil_cloned::cloned,
};

fn main() {
	console_error_panic_hook::set_once();
	tracing_subscriber::registry()
		.with(
			tracing_subscriber::fmt::layer()
				.with_ansi(false)
				.without_time()
				.with_level(false)
				.with_writer(tracing_web::MakeWebConsoleWriter::new().with_pretty_level()),
		)
		.init();

	match self::run() {
		Ok(()) => tracing::info!("Successfully initialized"),
		Err(err) => tracing::error!("Unable to start: {err:?}"),
	}
}

fn run() -> Result<(), AppError> {
	let ctx = DynatosWebCtx::new().expect("Unable to create dynatos web context");

	let location = Location::new(&ctx);
	let _location = ctx.store().provide(location);

	ctx.body().with_child(self::page(&ctx));

	Ok(())
}

fn page(ctx: &DynatosWebCtx) -> web_sys::HtmlElement {
	// TODO: If we add `.with_loadable_default()`, use it again in this example.
	let query = SingleQuery::<i32>::new(ctx, "a");
	let query = QuerySignal::new(ctx, query);

	html::div(ctx).with_children([
		#[cloned(query)]
		html::p(ctx).with_dyn_text(move || format!("{:?}", query.get_cloned())),
		html::hr(ctx),
		dynatos_web_router::anchor(ctx, "/?a=5").with_text("5"),
		html::br(ctx),
		dynatos_web_router::anchor(ctx, "/?a=7").with_text("7"),
		html::br(ctx),
		dynatos_web_router::anchor(ctx, "/?a=abc").with_text("abc"),
		html::br(ctx),
		#[cloned(query)]
		html::button(ctx)
			.with_event_listener::<ev!(click)>(ctx, move |_ev| {
				if let Loadable::Loaded(value) = &mut *query.borrow_mut() {
					*value += 1;
				}
			})
			.with_text("Add"),
		html::br(ctx),
		html::button(ctx)
			.with_event_listener::<ev!(click)>(ctx, move |_ev| {
				query.set(6);
			})
			.with_text("6"),
	])
}
