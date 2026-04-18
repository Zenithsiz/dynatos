//! Counter (SSR) Backend

// Features
#![feature(proc_macro_hygiene)]

// Imports
use {
	app_error::{AppError, Context},
	core::{
		net::{IpAddr, Ipv4Addr, SocketAddr},
		time::Duration,
	},
	tracing_subscriber::EnvFilter,
	url::Url,
};

#[tokio::main]
async fn main() -> Result<(), AppError> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::from_default_env())
		.init();

	// Then build the app
	let location = "https://localhost:8081"
		.parse::<Url>()
		.context("Location was an invalid url")?;
	let app = axum::Router::new().nest(
		"/ssr/",
		dynatos_web_ssr_server::axum::router(counter_ssr::attach, location, Duration::from_secs(30)),
	);

	let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8081);
	let listener = tokio::net::TcpListener::bind(addr)
		.await
		.context("Unable to create tcp listener")?;
	axum::serve(listener, app)
		.await
		.context("Unable to start http server")?;

	Ok(())
}
