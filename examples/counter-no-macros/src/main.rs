//! Counter example (without any macros)

// Imports
use {
	app_error::AppError,
	dynatos_reactive::{Signal, SignalBorrowMut, SignalGet, SignalSet},
	dynatos_web::{DynatosWebCtx, EventTargetWithListener, JsResultContext, NodeWithChildren, NodeWithText, ev, html},
	dynatos_web_reactive::NodeWithDynText,
	tracing_subscriber::prelude::*,
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

	let counter = self::counter(&ctx);
	ctx.body().append_child(&counter).context("Unable to append counter")?;

	Ok(())
}

fn counter(ctx: &DynatosWebCtx) -> web_sys::HtmlElement {
	let value = Signal::new(0);
	html::div(ctx).with_children([
		{
			let value = value.clone();
			html::button(ctx)
				.with_text("Clear")
				.with_event_listener::<ev!(click)>(move |_ev| value.set(0))
		},
		{
			let value = value.clone();
			html::button(ctx)
				.with_text("+")
				.with_event_listener::<ev!(click)>(move |_ev| *value.borrow_mut() += 1)
		},
		{
			let value = value.clone();
			html::button(ctx)
				.with_text("-")
				.with_event_listener::<ev!(click)>(move |_ev| *value.borrow_mut() -= 1)
		},
		html::span(ctx).with_dyn_text(move || format!("Value: {}.", value.get())),
	])
}
