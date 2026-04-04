#[test]
#[cfg(not(miri))]
fn compile_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/failure/*.rs");
}
