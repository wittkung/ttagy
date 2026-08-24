import test from "node:test";
import assert from "node:assert/strict";
import { AgyClient } from "../index.ts";

test("AgyClient can be initialized with default options", () => {
  const client = new AgyClient();
  assert.ok(client);
});

test("AgyClient builds request properly", async () => {
  const client = new AgyClient({ autoFallback: false, socketPath: "/non_existent_socket.sock" });
  const events = [];
  for await (const ev of client.streamChat({ prompt: "Hello" })) {
    events.push(ev);
  }
  assert.equal(events.length, 1);
  assert.equal(events[0].type, "agy:error");
  assert.equal(events[0].errorCode, "DAEMON_UNAVAILABLE");
});
