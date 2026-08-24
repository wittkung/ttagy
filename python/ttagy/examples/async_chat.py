import asyncio
from ttagy import TtagyClient, TtagyRequest, ContentDeltaEvent, DoneEvent

async def main():
    client = TtagyClient(socket_path="/tmp/ttagy.sock")

    req = TtagyRequest(
        prompt="Explain the difference between asyncio tasks and threads in Python",
        model="gemini-3.7-flash",
        effort="low"
    )

    print("⚡ Streaming response from Python SDK:")
    async for event in client.stream_chat(req):
        if isinstance(event, ContentDeltaEvent):
            print(event.text_delta, end="", flush=True)
        elif isinstance(event, DoneEvent):
            print(f"\n\n✅ Done in {event.elapsed_ms:.2f}ms")

if __name__ == "__main__":
    asyncio.run(main())
