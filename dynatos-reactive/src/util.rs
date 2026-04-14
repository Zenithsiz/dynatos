//! Utilities

// Imports
use dynatos_sync_types::SyncBounds;

// TODO: Allow the user to specify this a big better than just wasm_bindgen / tokio?
pub fn spawn_task<F: Future<Output = ()> + SyncBounds + 'static>(f: F) {
	cfg_select! {
		all(feature = "wasm-js-promise", feature = "tokio") => {
			compile_error! { "The `wasm-js-promise` and `tokio` features are mutually exclusive" }
		}

		feature = "wasm-js-promise" => wasm_bindgen_futures::spawn_local(f),
		feature = "tokio" => _ = tokio::task::spawn(f),

		_ => {
			compile_error! { "At least one of the `wasm-js-promise` and `tokio` features must be enabled" }
		}
	};
}
