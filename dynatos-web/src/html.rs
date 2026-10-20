//! HTML elements

use {
	crate::DynatosWebCtx,
	wasm_bindgen::{JsCast, JsValue},
};

/// Expands to `$value` if `$value` exists, else expands to `$default`
macro or_default_ty {
	($default:ty, $e:ty $(,)?) => {
		$e
	},
	($default:ty $(,)?) => {
		$default
	},
}

/// Declares all elements
macro decl_elements(
	$( $fn_name:ident $( : $ElTy:ty )? ),* $(,)?
) {
	$(
		#[must_use]
		pub fn $fn_name(ctx: &DynatosWebCtx) -> or_default_ty![web_sys::HtmlElement, $( $ElTy )?] {
			let el_name = stringify!($fn_name);
			let element = ctx.document().create_element_ns(Some(crate::HTML_NAMESPACE), el_name)
				.unwrap_or_else(|err| self::on_create_fail(&err, el_name));

			element.dyn_into().unwrap_or_else(|err| self::on_cast_fail(&err, el_name))
		}
	)*
}

/// Function called when creating an element fails
#[cold]
fn on_create_fail(err: &JsValue, el_name: &str) -> ! {
	panic!(
		"Unable to create element {el_name:?} on namespace {:?}: {err:?}",
		crate::HTML_NAMESPACE
	);
}

/// Function called when casting an element fails
#[cold]
fn on_cast_fail(element: &web_sys::Element, el_name: &str) -> ! {
	panic!(
		"Created element {el_name:?} on namespace {:?} was of the wrong type: {element:?}",
		crate::HTML_NAMESPACE
	);
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
	canvas: web_sys::HtmlCanvasElement,
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
	details: web_sys::HtmlDetailsElement,
	dfn,
	dialog: web_sys::HtmlDialogElement,
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
	img: web_sys::HtmlImageElement,
	input: web_sys::HtmlInputElement,
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
	textarea: web_sys::HtmlTextAreaElement,
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
