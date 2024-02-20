//! Function component builder for [`dynatos`]

// Exports
pub use dynatos_builder_macros::builder;

/// Missing prop
#[derive(Clone, Copy, Default, Debug)]
pub struct MissingProp;
