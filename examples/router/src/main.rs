//! Router example

// Features
#![feature(thread_local, stmt_expr_attributes, proc_macro_hygiene)]

// Imports
use {
	app_error::AppError,
	core::cell::OnceCell,
	dynatos_reactive::SignalGetCloned,
	dynatos_web::{DynatosWebCtx, NodeWithChildren, NodeWithText, html},
	dynatos_web_reactive::NodeWithDynChildren,
	dynatos_web_router::Location,
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
	let _location = ctx.store().provide(location.clone());

	ctx.body().with_child(
		html::div(&ctx)
			.with_children([html::p(&ctx).with_text("Header"), html::hr(&ctx)])
			.with_dyn_children(
				&ctx,
				#[cloned(ctx)]
				move || self::render_route(&ctx),
			)
			.with_children([
				html::hr(&ctx),
				dynatos_web_router::anchor(&ctx, location.clone(), "/test").with_text("Test"),
				html::br(&ctx),
				dynatos_web_router::anchor(&ctx, location.clone(), "/cached").with_text("Cached"),
				html::br(&ctx),
				dynatos_web_router::anchor(&ctx, location, "/empty").with_text("Empty"),
			]),
	);

	Ok(())
}

#[thread_local]
static ROUTE_CACHED: OnceCell<web_sys::HtmlElement> = OnceCell::new();


fn render_route(ctx: &DynatosWebCtx) -> Option<web_sys::HtmlElement> {
	let location = ctx.store().expect_cloned::<Location>().get_cloned();

	tracing::info!(%location, "Rendering route");
	match location.path().trim_end_matches('/') {
		// Always re-create page a
		"/test" => Some(self::page(ctx, "Test")),
		// Cache the 2nd route to show that `dyn_child` can handle the same element fine.
		"/cached" => {
			let route = ROUTE_CACHED.get_or_init(|| self::page(ctx, "Cached"));
			Some(route.clone())
		},
		// Have a page without any content
		"/empty" => None,
		// And finally a catch-all page
		page => Some(self::page(ctx, &format!("Unknown Page ({page:?})"))),
	}
}

fn page(ctx: &DynatosWebCtx, name: &str) -> web_sys::HtmlElement {
	tracing::info!(%name, "Rendering page");
	html::p(ctx).with_text(format!("Page {name}"))
}
