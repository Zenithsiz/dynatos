//! Thread local macro

cfg_select! {
	feature = "sync" => {
		pub macro thread_local_or_global {
			attr() ($i:item) => {
				$i
			},
		}
	},
	_ => {
		pub macro thread_local_or_global {
			attr() ($i:item) => {
				#[thread_local]
				$i
			},
		}
	},
}
