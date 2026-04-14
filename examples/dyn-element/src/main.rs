//! Dynamic child example

// Features
#![feature(stmt_expr_attributes, proc_macro_hygiene)]

// Imports
use {
	app_error::AppError,
	dynatos_reactive::{Signal, SignalGet, SignalSet, SignalUpdate},
	dynatos_web::{DynatosWebCtx, EventTargetWithListener, JsResultContext, NodeWithChildren, NodeWithText, ev, html},
	dynatos_web_reactive::DynElement,
	strum::VariantArray,
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
	let outer_el = Signal::new(Element::P);
	html::div(ctx)
		.with_child(
			html::div(ctx).with_children(
				Element::VARIANTS
					.iter()
					.map(|&el| {
						#[cloned(outer_el)]
						html::button(ctx)
							.with_text(format!("{el:?}"))
							.with_event_listener::<ev!(click)>(ctx, move |_| outer_el.set(el))
					})
					.collect::<Vec<_>>(),
			),
		)
		.with_child(html::hr(ctx))
		.with_child(self::outer(ctx, outer_el))
		.with_child(html::hr(ctx))
}

fn outer(ctx: &DynatosWebCtx, cur_el: Signal<Element>) -> DynElement {
	#[cloned(ctx)]
	let f = move || match cur_el.get() {
		Element::P => html::p(&ctx).with_child(self::inner(&ctx)),
		Element::A => html::a(&ctx).with_child(self::inner(&ctx)),
		Element::Pre => html::pre(&ctx).with_child(self::inner(&ctx)),
	};

	DynElement::new(ctx, f)
}

fn inner(ctx: &DynatosWebCtx) -> DynElement {
	let counter = Signal::new(0_usize);

	#[cloned(ctx)]
	let f = move || {
		let cur_counter = counter.get();
		let el = match cur_counter.is_multiple_of(2) {
			true => html::p(&ctx),
			false => html::b(&ctx),
		};

		#[cloned(counter)]
		el.with_text(cur_counter.to_string())
			.with_event_listener::<ev!(click)>(&ctx, move |_| counter.update(|counter| *counter += 1))
	};

	DynElement::new(ctx, f)
}

#[derive(Clone, Copy, Debug)]
#[derive(strum::VariantArray)]
enum Element {
	P,
	A,
	Pre,
}
