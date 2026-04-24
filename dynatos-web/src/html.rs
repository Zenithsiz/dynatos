//! HTML elements

// Imports
use crate::{
	DynatosWebCtx,
	types::{
		Element,
		HtmlCanvasElement,
		HtmlDialogElement,
		HtmlElement,
		HtmlImageElement,
		HtmlInputElement,
		HtmlTextAreaElement,
		WebError,
		cfg_ssr_expr,
	},
};

/// Expands to `$value` if `$value` exists, else expands to `$default`
macro or_default {
	($default:tt, $e:tt $(,)?) => {
		$e
	},
	($default:tt $(,)?) => {
		$default
	},
}

/// Declares all elements
macro decl_elements(
	$( $fn_name:ident $( : $ElTy:ty )? ),* $(,)?
) {
	$(
		#[must_use]
		pub fn $fn_name(ctx: &DynatosWebCtx) -> or_default![HtmlElement, $( $ElTy )?] {
			let el_name = stringify!($fn_name);
			let element = ctx.document().create_element_ns(Some(crate::HTML_NAMESPACE), el_name)
				.unwrap_or_else(|err| self::on_create_fail(&err, el_name));

			cfg_ssr_expr!(
				ssr = {
					use dynatos_inheritance::Downcast;
					element.downcast()
				},
				csr = {
					use wasm_bindgen::JsCast;
					element.dyn_into()
				},
			).unwrap_or_else(|err| self::on_cast_fail(&err, el_name))
		}
	)*
}

/// Function called when creating an element fails
#[cold]
fn on_create_fail(err: &WebError, el_name: &str) -> ! {
	panic!(
		"Unable to create element {el_name:?} on namespace {:?}: {err:?}",
		crate::HTML_NAMESPACE
	);
}

/// Function called when casting an element fails
#[cold]
fn on_cast_fail(element: &Element, el_name: &str) -> ! {
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
	canvas: HtmlCanvasElement,
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
	dialog: HtmlDialogElement,
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
	img: HtmlImageElement,
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
	textarea: HtmlTextAreaElement,
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
