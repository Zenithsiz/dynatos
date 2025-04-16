//! Counter example

// Features
#![feature(stmt_expr_attributes, proc_macro_hygiene)]

// Imports
use {
	dynatos_egui::EguiEffect,
	dynatos_reactive::{Signal, SignalGet, SignalSet, SignalUpdate, WorldGlobal},
	eframe::egui,
	std::time::Duration,
	zutil_cloned::cloned,
};


fn main() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt::init();

	let native_options = eframe::NativeOptions::default();
	eframe::run_native("Counter", native_options, Box::new(|cc| Ok(Box::new(EguiApp::new(cc)))))
		.map_err(|err| anyhow::anyhow!("Unable to start egui: {err}"))?;

	Ok(())
}

struct EguiApp {
	/// Value
	value: Signal<i32, WorldGlobal>,

	/// Egui effect
	effect: EguiEffect,
}

impl EguiApp {
	fn new(cc: &eframe::CreationContext<'_>) -> Self {
		Self {
			value:  Signal::new(0),
			effect: EguiEffect::new(cc.egui_ctx.clone()),
		}
	}
}

impl eframe::App for EguiApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		let _draw_deps = self.effect.deps_gatherer();

		egui::CentralPanel::default().show(ctx, |ui| {
			ui.horizontal(|ui| {
				if ui.button("clear").clicked() {
					self.value.set(0);
				}
				if ui.button("+").clicked() {
					self.value.update(|value| *value += 1);
				}
				if ui.button("-").clicked() {
					self.value.update(|value| *value -= 1);
				}
				ui.heading(format!("Value: {}", self.value.get()));
			});

			if ui.button("Start incrementing thread").clicked() {
				#[cloned(value = self.value)]
				std::thread::spawn(move || loop {
					value.update(|value| *value += 1);
					std::thread::sleep(Duration::from_secs_f32(0.1));
				});
			}
		});
	}
}
