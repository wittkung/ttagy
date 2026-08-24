import test from "node:test";
import assert from "node:assert/strict";
import * as path from "node:path";
import * as url from "node:url";
import { TtagyClient } from "../index.ts";

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const mockAgyPath = path.resolve(__dirname, "../../../../target/debug/mock-agy");

test("TS CTS: stream_normal parity", async () => {
  process.env.AGY_PATH = mockAgyPath;

  const client = new TtagyClient({ autoFallback: true });
  const events = [];
  let content = "";

  for await (const ev of client.streamChat({
    prompt: "scenario:stream_normal",
    model: "gemini-3.7-flash",
    effort: "low",
  })) {
    events.push(ev);
    if (ev.type === "agy:content_delta") {
      content += ev.textDelta;
    }
  }

  const types = events.map((e) => e.type);
  assert.deepEqual(types, [
    "agy:init",
    "agy:thinking_delta",
    "agy:content_delta",
    "agy:content_delta",
    "agy:done",
  ]);
  assert.equal(content, "你好，我是 Antigravity AI 助手。很高兴为您服务！");
});

test("TS CTS: structured_json parity", async () => {
  process.env.AGY_PATH = mockAgyPath;

  const client = new TtagyClient({ autoFallback: true });
  const result = await client.runJson({
    prompt: "scenario:structured_json",
    model: "gemini-3.7-flash",
    effort: "low",
  });

  assert.equal(result.status, "success");
  assert.equal(result.task, "compression");
  assert.equal(result.files_count, 3);
});

test("TS CTS: quota_error parity", async () => {
  process.env.AGY_PATH = mockAgyPath;

  const client = new TtagyClient({ autoFallback: true });
  const events = [];

  for await (const ev of client.streamChat({
    prompt: "scenario:quota_error",
    model: "gemini-3.7-flash",
    effort: "low",
  })) {
    events.push(ev);
  }

  assert.equal(events.length, 2);
  assert.equal(events[0].type, "agy:init");
  assert.equal(events[1].type, "agy:error");
  assert.ok(events[1].errorMessage.includes("Resource quota exceeded"));
});
