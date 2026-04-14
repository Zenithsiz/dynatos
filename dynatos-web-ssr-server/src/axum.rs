//! `Axum` backend

// Imports
use {
	crate::state::{self, State},
	app_error::{AppError, Context},
	axum::{
		extract,
		http::StatusCode,
		response::{Html, IntoResponse, Redirect, Response},
		routing,
	},
	axum_cookie::{CookieLayer, CookieManager, prelude::Cookie},
	core::time::Duration,
	dynatos_web::{
		DynatosWebCtx,
		JsResultContext,
		types::{EventTarget, PointerEvent},
	},
	std::thread,
	uuid::Uuid,
	zutil_cloned::cloned,
};

pub fn router(attach: fn(&DynatosWebCtx), max_alive: Duration) -> axum::Router {
	let state = State::new(attach);

	#[cloned(state;)]
	thread::spawn(move || state::garbage_collect_clients(&state.0.clients, max_alive));

	let route = routing::get(self::session).post(self::event);
	axum::Router::new()
		.route("/", route.clone())
		.route("/{*path}", route)
		.layer(CookieLayer::default())
		.with_state(state)
}

static CLIENT_ID_COOKIE_NAME: &str = "dynatos-client-uuid";

#[axum::debug_handler]
async fn session(
	cookies: CookieManager,
	path: Option<extract::Path<String>>,
	extract::State(state): extract::State<State>,
) -> Result<Response, ReqError> {
	let path = match path {
		Some(extract::Path(path)) => path,
		None => String::new(),
	};

	let client_id = match cookies.get(CLIENT_ID_COOKIE_NAME) {
		Some(client_id) => Uuid::try_parse(client_id.value()).context("Invalid client uuid")?,
		None => {
			let uuid = Uuid::new_v4();
			let cookie = Cookie::new(CLIENT_ID_COOKIE_NAME, uuid.to_string());
			cookies.add(cookie);

			uuid
		},
	};

	// Either create the new client, or navigate to the new location
	let location = format!("http://localhost:8081/{path}");
	let ctx = state
		.create_or_navigate_client(client_id, location)
		.context("Unable to get client")?;
	ctx.ssr_state().wait_all().await;

	// And finally render the html
	Ok(Html(crate::render_html(&ctx)).into_response())
}

#[derive(Debug)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ActionForm {
	handler_idx: usize,
}

#[axum::debug_handler]
async fn event(
	cookies: CookieManager,
	extract::State(state): extract::State<State>,
	extract::Form(form): extract::Form<ActionForm>,
) -> Result<Response, ReqError> {
	let client_id = cookies.get(CLIENT_ID_COOKIE_NAME).context("Missing cookie")?;
	let client_id = Uuid::try_parse(client_id.value()).context("Invalid client uuid")?;

	match state.client_ctx(client_id) {
		// If the client exists, then handle the event
		Some(ctx) => {
			let event = PointerEvent::default();
			EventTarget::handle(ctx.ssr_state(), form.handler_idx, event.into())
				.context("Unable to handle event target")?;
			ctx.ssr_state().wait_all().await;

			// TODO: Does redirecting to an empty string always
			//       use the current url, or do we need to explicitly
			//       set this to it? If so, we need to take into account
			//       reverse proxies to get the true location.
			Ok(Redirect::to("").into_response())
		},

		// Otherwise, their session timed out, so inform them and redirect them.
		_ => Ok(Html(crate::render_html_session_reset()).into_response()),
	}
}

/// Request error
pub struct ReqError(AppError);

impl<E: Into<AppError>> From<E> for ReqError {
	fn from(err: E) -> Self {
		Self(err.into())
	}
}

impl IntoResponse for ReqError {
	fn into_response(self) -> Response {
		let status = StatusCode::INTERNAL_SERVER_ERROR;
		let message = self.0.pretty().to_string();

		(status, message).into_response()
	}
}
