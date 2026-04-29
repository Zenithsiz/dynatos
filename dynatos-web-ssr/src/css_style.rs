//! Css style

// Imports
use {
	crate::{HtmlElement, Object, WebError, object::ObjectFields},
	app_error::app_error,
	zutil_inheritance::{FromFields, Value},
	itertools::Itertools,
	std::collections::HashMap,
};

zutil_inheritance::value! {
	pub struct CssStyleDeclaration(Object): Send + Sync + Debug {
		element: HtmlElement,
	}
	impl Self {}
}

impl CssStyleDeclaration {
	fn get_style(&self) -> String {
		self.fields().element.get_attribute("style").unwrap_or_default()
	}

	fn set_prop_map(&self, map: &HashMap<&str, &str>) {
		let style = map
			.iter()
			.format_with(";", |(&key, &value), f| f(&format_args!("{key}: {value}")))
			.to_string();

		if let Err(err) = self.fields().element.set_attribute("style", &style) {
			tracing::warn!("Unable to update `style` property: {err:?}");
		}
	}

	pub fn set_property(&self, name: &str, value: &str) -> Result<(), WebError> {
		let style = self.get_style();
		let mut css_props = self::parse_prop_map(&style);
		css_props.insert(name, value);
		self.set_prop_map(&css_props);

		Ok(())
	}

	pub fn remove_property(&self, name: &str) -> Result<String, WebError> {
		let style = self.get_style();
		let mut css_props = self::parse_prop_map(&style);
		match css_props.remove(name) {
			Some(value) => {
				self.set_prop_map(&css_props);
				Ok(value.to_owned())
			},
			None => Err(WebError(app_error!("Unknown css property: {name:?}"))),
		}
	}
}

zutil_inheritance::value! {
	pub struct CssStyleProperties(CssStyleDeclaration, Object): Send + Sync + Debug {

	}
	impl Self {}
}

impl CssStyleProperties {
	pub(crate) fn new(element: HtmlElement) -> Self {
		Self::from_fields((
			CssStylePropertiesFields {},
			CssStyleDeclarationFields { element },
			ObjectFields::default(),
		))
	}
}

fn parse_prop_map(style: &str) -> HashMap<&str, &str> {
	style
		.split(';')
		.filter_map(|prop| {
			let Some((key, value)) = prop.split_once(':') else {
				tracing::warn!("Ignoring malformed css attribute: {prop:?}");
				return None;
			};

			Some((key.trim(), value.trim()))
		})
		.collect::<HashMap<_, _>>()
}
