//! Import html example

// Features
#![feature(macro_metavar_expr, never_type, unboxed_closures, fn_traits)]

// Imports
use {
	app_error::AppError,
	dynatos_html::{JsResultContext, NodeWithChildren, NodeWithText, html, html_file},
	tracing_subscriber::prelude::*,
	web_sys::HtmlElement,
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

	let root = self::import_html();
	body.append_child(&root).context("Unable to append element")?;

	Ok(())
}

fn import_html() -> HtmlElement {
	let static_literal = html!(r#"<div>Static from literal</div>"#);

	let static_file = html_file!("import-html/src/pages/static.html");

	let element1 = html::p;
	let element2_value = html::p;

	let node1 = || html::p().with_text("Node 1");
	let node2_value = || html::p().with_text("Node 2");

	let attr1 = "my-attr1";
	let attr2_value = "my-attr2";
	let attr3 = "my-attr3";

	let ev1_capture = 1;
	let ev2_capture = 2;

	let ev1 = move |ev: web_sys::PointerEvent| tracing::warn!(?ev1_capture, "Event listener 1: {ev:?}");
	let ev2_value = move |ev: web_sys::Event| tracing::warn!(?ev2_capture, "Event listener 2: {ev:?}");

	let text1 = "my-text1";
	let text2_value = "my-text2";

	let dynamic =
		dynatos_html::parse_html_element(include_str!("pages/dynamic.html"), dynatos_html::parse::environment! {
			element {
				element1,
				element2: element2_value,
			}
			node {
				node1,
				node2: node2_value,
			}
			attr {
				attr1,
				attr2: attr2_value,
				attr3,
			}
			ev {
				ev1,
				ev2: ev2_value,
			}
			text {
				text1,
				text2: text2_value,
			}
		})
		.expect("Unable to parse html");

	html::div()
		.with_child(static_literal)
		.with_child(static_file)
		.with_child(dynamic)
}
