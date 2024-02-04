//! Logging helper
//!
//! Used for all other binaries to implement consistent logging

// Features
#![feature(array_chunks, array_windows, let_chains)]

// Imports
use tracing_subscriber::prelude::*;

/// Initializes logging
pub fn init() {
	// Create the registry
	let registry = tracing_subscriber::registry();

	// Add all the layers
	#[cfg(not(target_family = "wasm"))]
	let registry = {
		use {std::env, tracing::level_filters::LevelFilter};

		// Check if we should use colors
		// TODO: Check if we're being piped and disable by default?
		let log_use_color = env::var("RUST_LOG_COLOR").map_or(true, |value| {
			matches!(value.trim().to_uppercase().as_str(), "1" | "YES" | "TRUE")
		});

		let filter = tracing_subscriber::EnvFilter::builder()
			.with_default_directive(LevelFilter::INFO.into())
			.from_env_lossy();
		let layer = tracing_subscriber::fmt::layer()
			.with_ansi(log_use_color)
			.with_filter(filter);

		registry.with(layer)
	};

	#[cfg(target_family = "wasm")]
	let registry = {
		let layer = tracing_subscriber::fmt::layer()
			.with_ansi(false)
			.without_time()
			.with_level(false)
			.with_writer(tracing_web::MakeWebConsoleWriter::new().with_pretty_level());
		registry.with(layer)
	};

	// Finally initialize it
	registry.init();
}
