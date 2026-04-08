//! Dynamic children example

// Features
#![feature(stmt_expr_attributes, proc_macro_hygiene)]

// Imports
use {
	app_error::AppError,
	dynatos_reactive::{Signal, SignalBorrowMut, SignalGet, SignalSet},
	dynatos_web::{DynatosWebCtx, JsResultContext, NodeWithChildren, NodeWithText, html},
	dynatos_web_reactive::NodeWithDynChildren,
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

	let parent = self::parent(&ctx);
	ctx.body().append_child(&parent).context("Unable to append counter")?;

	Ok(())
}

fn parent(ctx: &DynatosWebCtx) -> web_sys::HtmlElement {
	let base = Signal::new(0);
	let num_children = Signal::new(2);

	html::div(ctx)
		.with_child(self::counter(ctx, "Number of children", num_children.clone()))
		.with_child(self::counter(ctx, "Base", base.clone()))
		.with_child(html::hr(ctx))
		.with_dyn_children(
			ctx,
			#[cloned(ctx)]
			move || self::child(&ctx, base.get(), num_children.get()),
		)
		.with_child(html::hr(ctx))
		.with_child(html::p(ctx).with_text("Footer"))
}

fn child(ctx: &DynatosWebCtx, base: i32, num_children: i32) -> Vec<web_sys::HtmlElement> {
	(0..num_children)
		.map(|idx| html::p(ctx).with_text(format!("{}", base + idx)))
		.collect()
}

fn counter(ctx: &DynatosWebCtx, name: &str, value: Signal<i32>) -> web_sys::HtmlElement {
	#[cloned(value)]
	let reset = move |_ev| value.set(0);
	#[cloned(value)]
	let add = move |_ev| *value.borrow_mut() += 1;
	#[cloned(value)]
	let sub = move |_ev| *value.borrow_mut() -= 1;

	html!(
		r#"<div>
			<button @click="reset">Reset</button>
			<button @click="add">+</button>
			<button @click="sub">-</button>
			<span>%{static name}%: %{value.get()}%.</span>
		</div>"#
	)
}
