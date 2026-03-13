//! Dynamic children example

// Features
#![feature(stmt_expr_attributes, proc_macro_hygiene)]

// Imports
use {
	app_error::AppError,
	dynatos_html::{EventTargetWithListener, JsResultContext, NodeWithChildren, NodeWithText, ev, html},
	dynatos_html_reactive::NodeWithDynChild,
	dynatos_reactive::{Signal, SignalBorrowMut, SignalGet, SignalSet, SignalUpdate},
	tracing_subscriber::prelude::*,
	web_sys::HtmlElement,
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
	let window = web_sys::window().expect("Unable to get window");
	let document = window.document().expect("Unable to get document");
	let body = document.body().expect("Unable to get document body");

	let parent = self::parent();
	body.append_child(&parent).context("Unable to append counter")?;

	Ok(())
}

fn parent() -> HtmlElement {
	let num_children = Signal::new(2);
	let bump = Signal::new(0);

	#[cloned(bump)]
	let bump_el = html::button()
		.with_text("Bump")
		.with_event_listener::<ev!(click)>(move |_| bump.update(|bump| *bump += 1));

	html::div()
		.with_child(self::counter(num_children.clone()))
		.with_child(bump_el)
		.with_child(html::hr())
		.with_dyn_child(move || {
			let _ = bump.get();
			self::child(num_children.get())
		})
		.with_child(html::hr())
		.with_child(html::p().with_text("Footer"))
}

fn child(num_children: usize) -> Vec<HtmlElement> {
	(0..num_children)
		.map(|idx| html::p().with_text(format!("{idx:?}")))
		.collect()
}

fn counter(value: Signal<usize>) -> HtmlElement {
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
			<span>Number of children: %{value.get()}%.</span>
		</div>"#
	)
}
