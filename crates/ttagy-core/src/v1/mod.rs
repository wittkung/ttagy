//! TTAgy Core V1 强类型契约模型与解析器 (Frozen Stability Partition)

pub mod model;
pub mod parser;
pub mod stderr_drainer;
pub mod types;

pub use model::{resolve_model_name, CANONICAL_ALIASES};
pub use parser::{NdjsonParser, ParsedChunk, ParsedStreamItem, UsageMetadata};
pub use stderr_drainer::{RollingBuffer, StderrDrainer};
pub use types::{TtagyRequest, TtagyResponse, TtagyStreamEvent, AGY_SUPPORTED_MODELS};
