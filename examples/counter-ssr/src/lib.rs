//! Counter (SSR) Shared library

// Features
#![feature(stmt_expr_attributes, proc_macro_hygiene)]

// Imports
use {
	dynatos_reactive::{Signal, SignalBorrowMut, SignalGet, SignalSet},
	dynatos_web::{DynatosWebCtx, EventTargetWithListener, NodeWithChildren, NodeWithText, ev, html},
	dynatos_web_reactive::NodeWithDynText,
	dynatos_web::types::HtmlElement,
	zutil_cloned::cloned,
};


fn counter(ctx: &DynatosWebCtx) -> HtmlElement {
	let value = Signal::new(0);

	#[cloned(value)]
	let clear = move |_ev| value.set(0);
	#[cloned(value)]
	let add = move |_ev| *value.borrow_mut() += 1;
	#[cloned(value)]
	let sub = move |_ev| *value.borrow_mut() -= 1;

	html::div(ctx).with_children([
		html::button(ctx)
			.with_text("Clear")
			.with_event_listener::<ev!(click)>(ctx, clear),
		html::button(ctx)
			.with_text("+")
			.with_event_listener::<ev!(click)>(ctx, add),
		html::button(ctx)
			.with_text("-")
			.with_event_listener::<ev!(click)>(ctx, sub),
		html::span(ctx).with_dyn_text(move || format!("Value: {}.", value.get())),
	])
}

pub fn attach(ctx: &DynatosWebCtx) {
	let counter = self::counter(ctx);
	ctx.body().append_child(&counter).expect("Unable to append counter");
}
