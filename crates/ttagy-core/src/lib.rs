//! TTAgy 核心库 - 强类型契约、沙箱隔离与解析管道 (Rust Pure Core)

pub mod detector;
pub mod sandbox;
pub mod v1;

pub use detector::TtagyDetector;
pub use sandbox::SandboxGuard;
pub use v1::*;
