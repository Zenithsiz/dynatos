//! Debug tests

// Features
#![feature(const_trait_impl, more_qualified_paths, trivial_bounds)]

// Imports
use dynatos_inheritance::FromFields;

dynatos_inheritance::value! {
	struct A(): Debug {
		a: String,
	}
	impl Self {}
}

dynatos_inheritance::value! {
	struct B(A): Debug {
		b0: String,
		b1: String,
	}
	impl Self {}
}

dynatos_inheritance::value! {
	struct C(B, A): Debug {
		c: String,
	}
	impl Self {}
}

impl C {
	const DEBUG_INNER: &str = r#"{ a: "a", b0: "b0", b1: "b1", c: "c" }"#;
}

impl Default for C {
	fn default() -> Self {
		Self::from_fields((
			CFields { c: "c".to_owned() },
			BFields {
				b0: "b0".to_owned(),
				b1: "b1".to_owned(),
			},
			AFields { a: "a".to_owned() },
		))
	}
}

#[test]
fn simple() {
	let c = C::default();
	assert_eq!(format!("{c:?}"), format!("C {}", C::DEBUG_INNER));
}

#[test]
fn downcast() {
	let c = C::default();
	let a = A::from(c);
	assert_eq!(format!("{a:?}"), format!("A {}", C::DEBUG_INNER));
}
