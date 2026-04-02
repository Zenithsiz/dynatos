//! Bounds necessary for synchronization

cfg_select! {
	feature = "sync" => { pub trait SyncBounds = Send + Sync; },
	_ => { pub trait SyncBounds = ; },
}
