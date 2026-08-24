use futures_util::StreamExt;
use ttagy_client::{ClientConfig, TtagyClient, TtagyRequest, TtagyStreamEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = TtagyClient::new(ClientConfig::default());

    let req = TtagyRequest::builder("Write a high-performance concurrent queue in Rust")
        .model("gemini-3.7-flash")
        .effort("low")
        .build();

    let mut stream = client.stream_chat(req).await?;

    while let Some(event_res) = stream.next().await {
        match event_res? {
            TtagyStreamEvent::ThinkingDelta { text_delta, .. } => {
                eprint!("{}", text_delta);
            }
            TtagyStreamEvent::ContentDelta { text_delta, .. } => {
                print!("{}", text_delta);
            }
            TtagyStreamEvent::Done { elapsed_ms, .. } => {
                println!("\n\n✅ Done in {:.2}ms", elapsed_ms);
            }
            TtagyStreamEvent::Error { error_message, .. } => {
                eprintln!("\n❌ Error: {}", error_message);
            }
            _ => {}
        }
    }

    Ok(())
}
