pub mod client;
pub mod fallback;

pub use client::{ClientBuilder, ClientConfig, TtagyClient};
pub use fallback::FallbackDriver;
pub use ttagy_core::{TtagyDetector, TtagyRequest, TtagyResponse, TtagyStreamEvent};
