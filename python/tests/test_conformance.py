import asyncio
import os
import unittest
from ttagy import (
    TtagyClient,
    TtagyRequest,
    InitEvent,
    ThinkingDeltaEvent,
    ContentDeltaEvent,
    DoneEvent,
    ErrorEvent,
)

class TestPythonConformance(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        # target/debug/mock-agy
        root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
        self.mock_agy_path = os.path.join(root, "target/debug/mock-agy")
        os.environ["AGY_PATH"] = self.mock_agy_path

    async def test_cts_stream_normal(self):
        client = TtagyClient(auto_fallback=True)
        events = []
        full_content = ""

        req = TtagyRequest(
            prompt="scenario:stream_normal",
            model="gemini-3.7-flash",
            effort="low",
        )

        async for ev in client.stream_chat(req):
            events.append(ev)
            if isinstance(ev, ContentDeltaEvent):
                full_content += ev.text_delta

        types = [type(e) for e in events]
        self.assertEqual(
            types,
            [
                InitEvent,
                ThinkingDeltaEvent,
                ContentDeltaEvent,
                ContentDeltaEvent,
                DoneEvent,
            ],
        )
        self.assertEqual(full_content, "你好，我是 Antigravity AI 助手。很高兴为您服务！")

    async def test_cts_structured_json(self):
        client = TtagyClient(auto_fallback=True)
        req = TtagyRequest(
            prompt="scenario:structured_json",
            model="gemini-3.7-flash",
            effort="low",
        )

        res = await client.run_json(req)
        self.assertEqual(res["status"], "success")
        self.assertEqual(res["task"], "compression")
        self.assertEqual(res["files_count"], 3)

    async def test_cts_quota_error(self):
        client = TtagyClient(auto_fallback=True)
        events = []

        req = TtagyRequest(
            prompt="scenario:quota_error",
            model="gemini-3.7-flash",
            effort="low",
        )

        async for ev in client.stream_chat(req):
            events.append(ev)

        self.assertEqual(len(events), 2)
        self.assertIsInstance(events[0], InitEvent)
        self.assertIsInstance(events[1], ErrorEvent)
        self.assertIn("Resource quota exceeded", events[1].error_message)

if __name__ == "__main__":
    unittest.main()
