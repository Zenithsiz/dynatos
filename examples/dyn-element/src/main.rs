//! Dynamic child example

// Features
#![feature(stmt_expr_attributes, proc_macro_hygiene)]

// Imports
use {
	app_error::AppError,
	dynatos_web::{EventTargetWithListener, JsResultContext, NodeWithChildren, NodeWithText, ev, html},
	dynatos_web_reactive::DynElement,
	dynatos_reactive::{Signal, SignalGet, SignalSet, SignalUpdate},
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
	let window = web_sys::window().expect("Unable to get window");
	let document = window.document().expect("Unable to get document");
	let body = document.body().expect("Unable to get document body");

	let parent = self::parent();
	body.append_child(&parent).context("Unable to append counter")?;

	Ok(())
}

fn parent() -> web_sys::HtmlElement {
	let outer_el = Signal::new(Element::P);
	html::div()
		.with_child(
			html::div().with_children(
				Element::VARIANTS
					.iter()
					.map(|&el| {
						#[cloned(outer_el)]
						html::button()
							.with_text(format!("{el:?}"))
							.with_event_listener::<ev!(click)>(move |_| outer_el.set(el))
					})
					.collect::<Vec<_>>(),
			),
		)
		.with_child(html::hr())
		.with_child(self::outer(outer_el))
		.with_child(html::hr())
}

fn outer(cur_el: Signal<Element>) -> DynElement {
	DynElement::new(move || match cur_el.get() {
		Element::P => html::p().with_child(self::inner()),
		Element::A => html::a().with_child(self::inner()),
		Element::Pre => html::pre().with_child(self::inner()),
	})
}

fn inner() -> DynElement {
	let counter = Signal::new(0_usize);

	DynElement::new(move || {
		let cur_counter = counter.get();
		let el = match cur_counter.is_multiple_of(2) {
			true => html::p(),
			false => html::b(),
		};

		#[cloned(counter)]
		el.with_text(cur_counter.to_string())
			.with_event_listener::<ev!(click)>(move |_| counter.update(|counter| *counter += 1))
	})
}

#[derive(Clone, Copy, Debug)]
#[derive(strum::VariantArray)]
enum Element {
	P,
	A,
	Pre,
}
