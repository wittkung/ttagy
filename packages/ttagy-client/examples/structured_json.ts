import { TtagyClient } from "../src/index.js";

interface SentimentAnalysis {
  sentiment: "positive" | "neutral" | "negative";
  confidence: number;
  key_phrases: string[];
}

async function main() {
  const client = new TtagyClient();

  const prompt = "Analyze the sentiment of: 'TTAgy brings sub-2ms response speed to Antigravity CLI.'";

  const result = await client.runJson<SentimentAnalysis>({
    prompt,
    model: "gemini-3.7-flash",
    jsonSchema: JSON.stringify({
      type: "object",
      required: ["sentiment", "confidence", "key_phrases"],
      properties: {
        sentiment: { type: "string", enum: ["positive", "neutral", "negative"] },
        confidence: { type: "number" },
        key_phrases: { type: "array", items: { type: "string" } },
      },
    }),
  });

  console.log("Structured JSON Output:", result);
}

main().catch(console.error);
