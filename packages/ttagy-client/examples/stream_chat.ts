import { TtagyClient } from "../src/index.js";

async function main() {
  const client = new TtagyClient({
    socketPath: "/tmp/ttagy.sock",
    autoFallback: true,
  });

  console.log("⚡ Starting streaming chat via TTAgy Client...");

  const stream = await client.streamChat({
    prompt: "Write a modern TypeScript discriminated union pattern",
    model: "gemini-3.7-flash",
    effort: "low",
  });

  for await (const event of stream) {
    if (event.type === "agy:thinking_delta") {
      process.stderr.write(`[Thinking] ${event.textDelta}`);
    } else if (event.type === "agy:content_delta") {
      process.stdout.write(event.textDelta);
    } else if (event.type === "agy:done") {
      console.log(`\n\n✅ Finished in ${event.elapsedMs}ms`);
    } else if (event.type === "agy:error") {
      console.error(`\n❌ Error: ${event.errorMessage}`);
    }
  }
}

main().catch(console.error);
