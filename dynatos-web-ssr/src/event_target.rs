//! Event target

// Imports
use {
	crate::{Element, Event, HtmlElement, Object, State, WebError},
	app_error::{AppError, Context, app_error},
	core::any::type_name,
	zutil_inheritance::{Downcast, Value},
};

#[derive(derive_more::Debug)]
#[debug("{_0:p}")]
pub(crate) struct Handler(Box<dyn Fn(Event) -> Result<(), AppError> + Send + Sync>);

zutil_inheritance::value! {
	pub struct EventTarget(Object): Send + Sync + Debug + Default {}
	impl Self {}
}

impl EventTarget {
	pub fn handle(state: &State, handler_idx: usize, event: Event) -> Result<(), WebError> {
		let event_target_handlers = state.0.event_target_handlers.lock();
		let handler = event_target_handlers
			.get(handler_idx)
			.context("Invalid handler index")?;

		(handler.0)(event)?;

		Ok(())
	}

	pub fn add_event_listener_with_callback<Ev, F>(&self, state: &State, name: &str, f: F) -> Result<(), WebError>
	where
		Ev: Value,
		F: Fn(Ev) + Send + Sync + 'static,
	{
		match name {
			"click" => {
				// TODO: Maybe restrict this to `HtmlButtonElement`?
				let el = self
					.downcast_ref::<Element>()
					.context("`click` event can only be added to `Element`s in SSR mode")?;

				// TODO: Should we use an uuid instead of an index?
				let mut handlers = state.0.event_target_handlers.lock();
				let handler_idx = handlers.len();
				let form_id = format!("dynatos-form-{handler_idx}");

				el.set_attribute("type", "submit")?;
				el.set_attribute("form", &form_id)?;

				let form = HtmlElement::new("form");
				form.set_attribute("action", "")?;
				form.set_attribute("method", "post")?;
				form.set_attribute("id", &form_id)?;
				state.body().append_child(&form)?;

				let data = HtmlElement::new("input");
				data.set_attribute("type", "hidden")?;
				data.set_attribute("form", &form_id)?;
				data.set_attribute("name", "handler-idx")?;
				data.set_attribute("value", &handler_idx.to_string())?;
				form.append_child(&data)?;

				let handler = move |ev: Event| {
					let ev = ev.downcast::<Ev>().map_err(|obj| {
						app_error!(
							"Event object was of the wrong type. Expected {}, found {obj:?}",
							type_name::<Ev>()
						)
					})?;

					f(ev);
					Ok(())
				};

				handlers.push(Handler(Box::new(handler)));
			},
			"popstate" => state.history().listen(move |ev| match ev.downcast::<Ev>() {
				Ok(ev) => f(ev),
				Err(ev) => tracing::warn!(
					"Event object was of the wrong type. Expected {}, found {ev:?}",
					type_name::<Ev>()
				),
			}),

			_ => return Err(WebError(app_error!("Unable to handle event: {name:?}"))),
		}

		Ok(())
	}
}
