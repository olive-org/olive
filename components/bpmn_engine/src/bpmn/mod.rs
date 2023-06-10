//! # BPMN Document
//!

pub use bpmn_schema as schema;
mod parser;
pub use parser::{parse, NormalizationError, ParseError};