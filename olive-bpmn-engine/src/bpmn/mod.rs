//! # BPMN Document
//!

pub use olive_bpmn_schema as schema;
mod parser;
pub use parser::{parse, NormalizationError, ParseError};