//! Egui effect

// Imports
use dynatos_reactive::{effect::EffectDepsGatherer, Effect};

/// An effect that redraws an egui context whenever any signals change
pub struct EguiEffect {
	/// Effect
	effect: Effect<EffectFn>,
}

impl EguiEffect {
	/// Creates a new egui effect from an egui context
	#[must_use]
	#[track_caller]
	pub fn new(ctx: egui::Context) -> Self {
		Self {
			effect: Effect::new_raw(effect_fn(ctx)),
		}
	}

	/// Returns the effect dependency gatherer.
	///
	/// You should call this within `draw`, and keep it around until you finish drawing.
	#[must_use]
	pub fn deps_gatherer(&self) -> EffectDepsGatherer {
		self.effect.deps_gatherer()
	}
}

use effect_fn::*;
mod effect_fn {
	pub type EffectFn = impl Fn();
	pub fn effect_fn(ctx: egui::Context) -> EffectFn {
		move || {
			tracing::debug!("Request redraw");
			ctx.request_repaint();
		}
	}
}
