import unittest
from ttagy.parser import NdjsonParser, ParsedThinkingDelta, ParsedContentDelta, ParsedDone, ParsedError

class TestNdjsonParser(unittest.TestCase):
    def test_step_update_composite(self):
        line = '{"type":"step_update","step_update":{"thought_delta":"思考中","text_delta":"你好"}}'
        items = NdjsonParser.parse_line_items(line)
        self.assertEqual(len(items), 2)
        self.assertEqual(items[0], ParsedThinkingDelta("思考中"))
        self.assertEqual(items[1], ParsedContentDelta("你好"))

    def test_message_nesting(self):
        line = '{"type":"message","message":{"content":"嵌套消息"}}'
        items = NdjsonParser.parse_line_items(line)
        self.assertEqual(len(items), 1)
        self.assertEqual(items[0], ParsedContentDelta("嵌套消息"))

    def test_result_with_usage(self):
        line = '{"type":"result","result":{"content":"完成","usage":{"prompt_tokens":10,"completion_tokens":2}}}'
        items = NdjsonParser.parse_line_items(line)
        self.assertEqual(len(items), 1)
        self.assertIsInstance(items[0], ParsedDone)
        self.assertEqual(items[0].content, "完成")
        self.assertEqual(items[0].prompt_tokens, 10)
        self.assertEqual(items[0].output_tokens, 2)

    def test_top_level_error(self):
        line = '{"type":"error","error":"Quota exceeded"}'
        items = NdjsonParser.parse_line_items(line)
        self.assertEqual(len(items), 1)
        self.assertEqual(items[0], ParsedError("CLI_ERROR", "Quota exceeded"))

if __name__ == "__main__":
    unittest.main()
