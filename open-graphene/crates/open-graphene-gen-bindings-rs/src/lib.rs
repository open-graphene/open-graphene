pub mod error;
pub mod generate;

pub use error::{GenBindingsRsError, Result};
pub use generate::{GenerateBindingsResult, generate_bindings};
