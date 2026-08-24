package ttagy_test

import (
	"context"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/wittkung/ttagy/golang/ttagy"
)

func getMockAgyPath(t *testing.T) string {
	wd, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	root := filepath.Clean(filepath.Join(wd, "../.."))
	p := filepath.Join(root, "target", "debug", "mock-agy")
	if _, err := os.Stat(p); err != nil {
		t.Fatalf("mock-agy not found at %s: %v", p, err)
	}
	return p
}

func TestExtractStructuredJSON(t *testing.T) {
	raw := "Here is JSON:\n```json\n{\n  \"status\": \"ok\",\n  \"count\": 10\n}\n```\nDone."
	extracted, err := ttagy.ExtractStructuredJSON(raw)
	if err != nil {
		t.Fatalf("ExtractStructuredJSON failed: %v", err)
	}
	if !strings.Contains(extracted, `"status": "ok"`) {
		t.Fatalf("unexpected extracted content: %s", extracted)
	}
}

func TestCTSStreamNormal(t *testing.T) {
	mockPath := getMockAgyPath(t)
	os.Setenv("AGY_PATH", mockPath)

	client := ttagy.NewClient(ttagy.ClientConfig{AutoFallback: true})
	ctx := context.Background()

	events, errs := client.StreamChat(ctx, ttagy.Request{
		Prompt: "scenario:stream_normal",
		Model:  "gemini-3.7-flash",
		Effort: "low",
	})

	var eventTypes []ttagy.StreamEventType
	var fullContent strings.Builder

	for events != nil || errs != nil {
		select {
		case ev, ok := <-events:
			if !ok {
				events = nil
				break
			}
			eventTypes = append(eventTypes, ev.Type)
			if ev.Type == ttagy.EventContentDelta {
				fullContent.WriteString(ev.TextDelta)
			}
		case err, ok := <-errs:
			if !ok {
				errs = nil
				break
			}
			if err != nil {
				t.Fatalf("StreamChat error: %v", err)
			}
		}
	}

	if len(eventTypes) < 4 {
		t.Fatalf("expected at least 4 events, got %v", eventTypes)
	}
	if !strings.Contains(fullContent.String(), "Antigravity AI 助手") {
		t.Fatalf("unexpected content: %s", fullContent.String())
	}
}

func TestCTSStructuredJSON(t *testing.T) {
	mockPath := getMockAgyPath(t)
	os.Setenv("AGY_PATH", mockPath)

	client := ttagy.NewClient(ttagy.ClientConfig{AutoFallback: true})
	ctx := context.Background()

	var result struct {
		Status     string `json:"status"`
		Task       string `json:"task"`
		FilesCount int    `json:"files_count"`
	}

	err := client.RunJSON(ctx, ttagy.Request{
		Prompt: "scenario:structured_json",
		Model:  "gemini-3.7-flash",
		Effort: "low",
	}, &result)

	if err != nil {
		t.Fatalf("RunJSON failed: %v", err)
	}
	if result.Status != "success" || result.Task != "compression" || result.FilesCount != 3 {
		t.Fatalf("unexpected result struct: %+v", result)
	}
}

func TestCTSQuotaError(t *testing.T) {
	mockPath := getMockAgyPath(t)
	os.Setenv("AGY_PATH", mockPath)

	client := ttagy.NewClient(ttagy.ClientConfig{AutoFallback: true})
	ctx := context.Background()

	events, _ := client.StreamChat(ctx, ttagy.Request{
		Prompt: "scenario:quota_error",
		Model:  "gemini-3.7-flash",
		Effort: "low",
	})

	var gotError bool
	var errorMsg string

	for ev := range events {
		if ev.Type == ttagy.EventError {
			gotError = true
			errorMsg = ev.ErrorMessage
		}
	}

	if !gotError {
		t.Fatal("expected EventError")
	}
	if !strings.Contains(errorMsg, "Resource quota exceeded") {
		t.Fatalf("unexpected error message: %s", errorMsg)
	}
}
