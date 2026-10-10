//! `dynatos` web SSR server

#![feature(nonpoison_mutex, nonpoison_condvar, sync_nonpoison)]
#![cfg_attr(feature = "axum", feature(proc_macro_hygiene, thread_sleep_until))]

#[cfg(feature = "axum")]
pub mod axum;
mod state;

pub use self::state::State;

use dynatos_web::DynatosWebCtx;

/// Renders a client's html
fn render_html(ctx: &DynatosWebCtx) -> String {
	let head = &***ctx.head();
	let body = &***ctx.body();
	format!("<!doctype html><html>{head}{body}</html>")
}

/// Renders a session reset html
const fn render_html_session_reset() -> &'static str {
	// TODO: This should be customizable
	r#"<!doctype html><html><head><meta http-equiv="refresh" content="2"/></head><body>Your session expired, redirecting in 2s</body></html>"#
}
