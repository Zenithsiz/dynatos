//! Counter (SSR) Frontend

// Imports
use {
	app_error::{AppError, Context},
	dynatos_web::DynatosWebCtx,
	tracing_subscriber::prelude::*,
};

fn main() {
	console_error_panic_hook::set_once();
	tracing_subscriber::registry()
		.with(
			tracing_subscriber::fmt::layer()
				.with_ansi(false)
				.without_time()
				.with_level(false)
				.with_writer(tracing_web::MakeWebConsoleWriter::new().with_pretty_level()),
		)
		.init();

	match self::run() {
		Ok(()) => tracing::info!("Successfully initialized"),
		Err(err) => tracing::error!("Unable to start: {err:?}"),
	}
}

fn run() -> Result<(), AppError> {
	let ctx = DynatosWebCtx::new().context("Unable to create dynatos web context")?;
	counter_ssr::attach(&ctx);

	Ok(())
}
