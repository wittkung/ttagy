import unittest
from ttagy import TtagyClient, TtagyRequest, extract_structured_json

class TestTtagyClient(unittest.TestCase):
    def test_client_initialization(self):
        client = TtagyClient(
            base_url="http://127.0.0.1:8970",
            socket_path="/tmp/ttagy.sock",
            auth_token="secret",
            auto_fallback=True,
        )
        self.assertEqual(client.base_url, "http://127.0.0.1:8970")
        self.assertEqual(client.socket_path, "/tmp/ttagy.sock")
        self.assertEqual(client.auth_token, "secret")
        self.assertTrue(client.auto_fallback)

    def test_request_to_dict(self):
        req = TtagyRequest(
            prompt="测试",
            model="gemini-3.7-flash",
            effort="high",
            temperature=0.7,
        )
        d = req.to_dict()
        self.assertEqual(d["prompt"], "测试")
        self.assertEqual(d["model"], "gemini-3.7-flash")
        self.assertEqual(d["effort"], "high")
        self.assertEqual(d["temperature"], 0.7)

    def test_extract_structured_json(self):
        markdown = 'Here is the result:\n```json\n{\n  "status": "ok",\n  "count": 42\n}\n```\nDone!'
        extracted = extract_structured_json(markdown)
        self.assertIn('"status": "ok"', extracted)
        self.assertIn('"count": 42', extracted)

if __name__ == "__main__":
    unittest.main()
