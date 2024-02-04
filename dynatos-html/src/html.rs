//! HTML elements

// Imports
use web_sys::Element;

/// Declares all elements
macro decl_elements(
	$( $fn_name:ident = $el_name:literal ),* $(,)?
) {
	$(
		pub fn $fn_name() -> Element {
			// TODO: Cache the document in a thread local?
			let window = web_sys::window().expect("Unable to get window");
			let document = window.document().expect("Unable to get document");
			document.create_element($el_name).expect(concat!("Unable to create element: `", $el_name, "`"))
		}
	)*
}

decl_elements! {
	a = "a",
	br = "br",
	button = "button",
	div = "div",
	hr = "hr",
	p = "p",
	span = "span",
}
