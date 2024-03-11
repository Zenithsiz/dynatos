//! HTML elements

// Imports
use {
	wasm_bindgen::{JsCast, JsValue},
	web_sys::{Element, HtmlInputElement},
};

/// Html namespace
const HTML_NAMESPACE: &str = "http://www.w3.org/1999/xhtml";

/// Expands to `stringify!($el_name)`, if present, otherwise to `$fn_name`
macro el_name {
	($fn_name:ident, $el_name:literal) => {
		$el_name
	},
	($fn_name:ident) => {
		stringify!($fn_name)
	},
}

/// Expands to `$ElTy`, if present, otherwise to `Element`
macro el_ty {
	($ElTy:ty) => {
		$ElTy
	},
	() => {
		Element
	},
}

/// Declares all elements
macro decl_elements(
	$( $fn_name:ident $( : $ElTy:ty )? $( = $el_name:literal )? ),* $(,)?
) {
	$(
		#[must_use]
		pub fn $fn_name() -> el_ty![$( $ElTy )?] {
			// TODO: Cache the document in a thread local?
			let window = web_sys::window().expect("Unable to get window");
			let document = window.document().expect("Unable to get document");

			let el_name = el_name!($fn_name $(, $el_name)?);
			document.create_element_ns(Some(HTML_NAMESPACE), el_name)
				.unwrap_or_else(|err| self::on_create_fail(&err, el_name))
				.dyn_into()
				.expect("Element was of the incorrect type")
		}
	)*
}

/// Function called when creating an element fails
fn on_create_fail(err: &JsValue, el_name: &str) -> ! {
	panic!("Unable to create element {el_name:?} on namespace {HTML_NAMESPACE:?}: {err:?}");
}

decl_elements! {
	a,
	abbr,
	acronym,
	address,
	area,
	article,
	aside,
	audio,
	b,
	base,
	bdi,
	bdo,
	big,
	blockquote,
	body,
	br,
	button,
	canvas,
	caption,
	center,
	cite,
	code,
	col,
	colgroup,
	content,
	data,
	datalist,
	dd,
	del,
	details,
	dfn,
	dialog,
	dir,
	div,
	dl,
	dt,
	em,
	embed,
	fieldset,
	figcaption,
	figure,
	font,
	footer,
	form,
	frame,
	frameset,
	h1,
	h2,
	h3,
	h4,
	h5,
	h6,
	head,
	header,
	hgroup,
	hr,
	html,
	i,
	iframe,
	image,
	img,
	input: HtmlInputElement,
	ins,
	kbd,
	label,
	legend,
	li,
	link,
	main,
	map,
	mark,
	marquee,
	menu,
	menuitem,
	meta,
	meter,
	nav,
	nobr,
	noembed,
	noframes,
	noscript,
	object,
	ol,
	optgroup,
	option,
	output,
	p,
	param,
	picture,
	plaintext,
	portal,
	pre,
	progress,
	q,
	rb,
	rp,
	rt,
	rtc,
	ruby,
	s,
	samp,
	script,
	search,
	section,
	select,
	shadow,
	slot,
	small,
	source,
	span,
	strike,
	strong,
	style,
	sub,
	summary,
	sup,
	table,
	tbody,
	td,
	template,
	textarea,
	tfoot,
	th,
	thead,
	time,
	title,
	tr,
	track,
	tt,
	u,
	ul,
	var,
	video,
	wbr,
	xmp,
}
