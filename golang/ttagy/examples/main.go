package main

import (
	"context"
	"fmt"
	"time"

	"github.com/wittkung/ttagy/golang/ttagy"
)

func main() {
	client := ttagy.NewClient(ttagy.ClientConfig{
		SocketPath:   "/tmp/ttagy.sock",
		AutoFallback: true,
	})

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	fmt.Println("⚡ Streaming tokens via Go SDK:")

	ch, err := client.StreamChat(ctx, ttagy.Request{
		Prompt: "Write an idiomatic Go worker pool pattern using channels",
		Model:  "gemini-3.7-flash",
		Effort: "low",
	})
	if err != nil {
		panic(err)
	}

	for ev := range ch {
		if ev.Type == ttagy.EventContentDelta {
			fmt.Print(ev.TextDelta)
		} else if ev.Type == ttagy.EventDone {
			fmt.Printf("\n\n✅ Done in %.2fms\n", ev.ElapsedMs)
		}
	}
}
