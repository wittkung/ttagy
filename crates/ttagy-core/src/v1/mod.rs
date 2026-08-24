//! TTAgy Core V1 强类型契约模型与解析器 (Frozen Stability Partition)

pub mod parser;
pub mod types;

pub use parser::{NdjsonParser, ParsedChunk};
pub use types::{TtagyRequest, TtagyResponse, TtagyStreamEvent};
