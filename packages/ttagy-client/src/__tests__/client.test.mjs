import test from "node:test";
import assert from "node:assert/strict";
import { TtagyClient } from "../index.ts";

test("TtagyClient initializes with remote node configuration", () => {
  const client = new TtagyClient({
    baseUrl: "http://127.0.0.1:8970",
    authToken: "secret123",
    autoFallback: true,
  });
  assert.ok(client);
});

test("TtagyClient errors gracefully when remote node unreachable without fallback", async () => {
  const client = new TtagyClient({
    baseUrl: "http://127.0.0.1:9999", // 不存在的端口
    authToken: "secret123",
    autoFallback: false,
  });
  const events = [];
  for await (const ev of client.streamChat({ prompt: "Hello" })) {
    events.push(ev);
  }
  assert.equal(events.length, 1);
  assert.equal(events[0].type, "agy:error");
  assert.equal(events[0].errorCode, "REMOTE_NODE_FAILED");
});
