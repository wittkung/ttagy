import test from "node:test";
import assert from "node:assert/strict";
import { TtagyClient, extractStructuredJson, repairIncompleteJson } from "../index.ts";

test("TtagyClient initializes with remote node configuration", () => {
  const client = new TtagyClient({
    baseUrl: "http://127.0.0.1:8970",
    socketPath: "/tmp/ttagy.sock",
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

test("extractStructuredJson strips markdown fences and parses generic JSON", () => {
  const markdown = "Here is the result:\n```json\n{\n  \"action\": \"compress\",\n  \"files\": [\"a.txt\", \"b.txt\"]\n}\n```\nDone!";
  const extracted = extractStructuredJson(markdown);
  const parsed = JSON.parse(extracted);
  assert.equal(parsed.action, "compress");
  assert.equal(parsed.files.length, 2);
});

test("extractStructuredJson handles escaped quotes inside strings", () => {
  const raw = `{"text": "Hello \\"World\\"", "valid": true}`;
  const extracted = extractStructuredJson(raw);
  const parsed = JSON.parse(extracted);
  assert.equal(parsed.text, 'Hello "World"');
  assert.equal(parsed.valid, true);
});

test("repairIncompleteJson closes unclosed structures", () => {
  const partial = '{"users": [{"name": "Alice"}, {"name": "Bob"';
  const repaired = repairIncompleteJson(partial);
  const parsed = JSON.parse(repaired);
  assert.equal(parsed.users.length, 2);
  assert.equal(parsed.users[1].name, "Bob");
});
