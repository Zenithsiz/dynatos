//! State

// Imports
use {
	app_error::{AppError, Context},
	core::time::Duration,
	dynatos_web::{DynatosWebCtx, JsResultContext},
	dynatos_web_ssr::JsValue,
	std::{
		collections::{HashMap, hash_map},
		sync::{
			Arc,
			nonpoison::{Condvar, Mutex},
		},
		thread,
		time::Instant,
	},
	uuid::Uuid,
};

#[derive(Debug)]
pub struct ClientState {
	pub ctx:         DynatosWebCtx,
	pub last_update: Instant,
}

#[derive(Debug)]
pub struct ClientStates {
	pub states: Mutex<HashMap<Uuid, ClientState>>,

	/// Condition variable to wait while the states are empty
	pub wait_while_empty: Condvar,
}

#[derive(Debug)]
pub struct Inner {
	pub clients: ClientStates,
	pub attach:  fn(&DynatosWebCtx),
}

#[derive(Clone, Debug)]
pub struct State(pub Arc<Inner>);

impl State {
	/// Creates a new server state
	pub fn new(attach: fn(&DynatosWebCtx)) -> Self {
		let inner = Inner {
			clients: ClientStates {
				states:           Mutex::new(HashMap::new()),
				wait_while_empty: Condvar::new(),
			},
			attach,
		};

		Self(Arc::new(inner))
	}

	/// Gets a client's context
	#[must_use]
	pub fn client_ctx(&self, client_id: Uuid) -> Option<DynatosWebCtx> {
		let client_states = self.0.clients.states.lock();
		client_states
			.get(&client_id)
			.map(|client_state| client_state.ctx.clone())
	}

	/// Creates a client at a location or navigates.
	///
	/// Returns the client's context.
	pub fn create_or_navigate_client(&self, client_id: Uuid, location: String) -> Result<DynatosWebCtx, AppError> {
		let mut client_states = self.0.clients.states.lock();
		let ctx = match client_states.entry(client_id) {
			hash_map::Entry::Occupied(entry) => {
				tracing::debug!(%client_id, ?location, "Navigating client");

				let client_state = entry.into_mut();
				client_state.last_update = Instant::now();
				let ctx = client_state.ctx.clone();
				drop(client_states);

				ctx.history()
					.push_state_with_url(&JsValue::UNDEFINED, "", Some(&location))
					.context("Unable to update location")?;
				ctx
			},
			hash_map::Entry::Vacant(entry) => {
				tracing::debug!(%client_id, ?location, "Creating client");

				let ssr_state = dynatos_web_ssr::State::new(location);
				let ctx = DynatosWebCtx::new(ssr_state).context("Unable to create web context")?;

				entry.insert(ClientState {
					ctx:         ctx.clone(),
					last_update: Instant::now(),
				});
				drop(client_states);
				self.0.clients.wait_while_empty.notify_all();

				(self.0.attach)(&ctx);
				ctx
			},
		};
		Ok(ctx)
	}
}

pub fn garbage_collect_clients(clients: &ClientStates, max_alive: Duration) -> ! {
	let mut client_states = clients.states.lock();
	loop {
		let mut deadline = None::<Instant>;
		let mut update_deadline = |new_deadline: Instant| match &mut deadline {
			Some(deadline) =>
				if new_deadline < *deadline {
					*deadline = new_deadline;
				},
			None => deadline = Some(new_deadline),
		};

		let now = Instant::now();
		client_states.retain(|client_id, client_state| {
			let alive = now.saturating_duration_since(client_state.last_update);

			let retain = alive <= max_alive;
			match retain {
				true => update_deadline(client_state.last_update + max_alive),
				false => tracing::debug!(%client_id, "Garbage collecting client"),
			}

			retain
		});

		match deadline {
			// If we have the deadline, sleep until it
			Some(deadline) => {
				drop(client_states);
				thread::sleep_until(deadline);
				client_states = clients.states.lock();
			},

			// Otherwise, wait until another client is added to
			// get a deadline for them.
			None => clients.wait_while_empty.wait(&mut client_states),
		}
	}
}
