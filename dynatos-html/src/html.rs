//! HTML elements

// Imports
use {wasm_bindgen::JsValue, web_sys::Element};

/// Expands to `stringify!($el_name)`, if present, otherwise to `$fn_name`
macro el_name {
	($fn_name:ident, $el_name:literal) => {
		$el_name
	},
	($fn_name:ident) => {
		stringify!($fn_name)
	},
}

/// Declares all elements
macro decl_elements(
	$( $fn_name:ident $( = $el_name:literal )? ),* $(,)?
) {
	$(
		pub fn $fn_name() -> Element {
			// TODO: Cache the document in a thread local?
			let window = web_sys::window().expect("Unable to get window");
			let document = window.document().expect("Unable to get document");

			let el_name = el_name!($fn_name $(, $el_name)?);
			document.create_element( el_name )
				.unwrap_or_else(|err| self::on_create_fail(err, el_name))
		}
	)*
}

/// Function called when `document.create_element` fails
fn on_create_fail(err: JsValue, el_name: &str) -> ! {
	panic!("Unable to create element {el_name:?}: {err:?}");
}

decl_elements! {
	a,
	br,
	button,
	div,
	hr,
	p,
	span,
	template,
}
