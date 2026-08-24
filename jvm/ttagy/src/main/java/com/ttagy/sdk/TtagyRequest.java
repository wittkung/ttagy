package com.ttagy.sdk;

public record TtagyRequest(
    String sessionId,
    String prompt,
    String model,
    String effort,
    Double temperature,
    String systemInstruction,
    String jsonSchema,
    int timeoutSecs
) {
    public static Builder builder(String prompt) {
        return new Builder(prompt);
    }

    public static class Builder {
        private String sessionId;
        private final String prompt;
        private String model = "gemini-3.7-flash";
        private String effort = "low";
        private Double temperature;
        private String systemInstruction;
        private String jsonSchema;
        private int timeoutSecs = 60;

        public Builder(String prompt) {
            this.prompt = prompt;
        }

        public Builder sessionId(String sessionId) { this.sessionId = sessionId; return this; }
        public Builder model(String model) { this.model = model; return this; }
        public Builder effort(String effort) { this.effort = effort; return this; }
        public Builder temperature(double temperature) { this.temperature = temperature; return this; }
        public Builder systemInstruction(String systemInstruction) { this.systemInstruction = systemInstruction; return this; }
        public Builder jsonSchema(String jsonSchema) { this.jsonSchema = jsonSchema; return this; }
        public Builder timeoutSecs(int timeoutSecs) { this.timeoutSecs = timeoutSecs; return this; }

        public TtagyRequest build() {
            return new TtagyRequest(sessionId, prompt, model, effort, temperature, systemInstruction, jsonSchema, timeoutSecs);
        }
    }
}
