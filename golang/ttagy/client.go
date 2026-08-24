package ttagy

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"syscall"
	"time"
)

// RollingBuffer 固定容量的线程安全环形日志缓冲区
type RollingBuffer struct {
	mu           sync.Mutex
	buf          []byte
	maxBytes     int
	totalDropped int
}

func NewRollingBuffer(maxBytes int) *RollingBuffer {
	return &RollingBuffer{
		buf:      make([]byte, 0, maxBytes),
		maxBytes: maxBytes,
	}
}

func (rb *RollingBuffer) Write(p []byte) (n int, err error) {
	rb.mu.Lock()
	defer rb.mu.Unlock()

	n = len(p)
	if n >= rb.maxBytes {
		rb.totalDropped += len(rb.buf) + (n - rb.maxBytes)
		rb.buf = make([]byte, rb.maxBytes)
		copy(rb.buf, p[n-rb.maxBytes:])
		return n, nil
	}

	overflow := (len(rb.buf) + n) - rb.maxBytes
	if overflow > 0 {
		rb.totalDropped += overflow
		rb.buf = rb.buf[overflow:]
	}
	rb.buf = append(rb.buf, p...)
	return n, nil
}

func (rb *RollingBuffer) String() string {
	rb.mu.Lock()
	defer rb.mu.Unlock()

	if rb.totalDropped > 0 {
		return fmt.Sprintf("[... 截断前置 %d 字节 ...]\n%s", rb.totalDropped, string(rb.buf))
	}
	return string(rb.buf)
}

func DrainStderrAsync(r io.Reader, maxBytes int) (*RollingBuffer, <-chan struct{}) {
	rb := NewRollingBuffer(maxBytes)
	done := make(chan struct{})

	go func() {
		defer close(done)
		buf := make([]byte, 4096)
		for {
			n, err := r.Read(buf)
			if n > 0 {
				_, _ = rb.Write(buf[:n])
			}
			if err != nil {
				break
			}
		}
	}()

	return rb, done
}

func buildGuardedCommand(ctx context.Context, binary string, args []string, dir string) *exec.Cmd {
	cmd := exec.CommandContext(ctx, binary, args...)
	cmd.Dir = dir
	cmd.SysProcAttr = &syscall.SysProcAttr{
		Setpgid: true,
	}
	cmd.Cancel = func() error {
		if cmd.Process != nil && cmd.Process.Pid > 0 {
			return syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL)
		}
		return nil
	}
	cmd.WaitDelay = 2 * time.Second
	return cmd
}

func NewUDSHTTPClient(socketPath string) *http.Client {
	return &http.Client{
		Transport: &http.Transport{
			DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				var d net.Dialer
				return d.DialContext(ctx, "unix", socketPath)
			},
			DisableKeepAlives:   false,
			MaxIdleConns:        16,
			IdleConnTimeout:     90 * time.Second,
			TLSHandshakeTimeout: 0,
		},
		Timeout: 0,
	}
}

// Client TTAgy Go SDK 客户端
type Client struct {
	config     ClientConfig
	httpClient *http.Client
}

func NewClient(cfg ClientConfig) *Client {
	if cfg.SocketPath == "" && cfg.BaseURL == "" {
		cfg.SocketPath = "/tmp/ttagy.sock"
	}
	var httpClient *http.Client
	if cfg.SocketPath != "" {
		httpClient = NewUDSHTTPClient(cfg.SocketPath)
	} else {
		httpClient = &http.Client{Timeout: 0}
	}
	return &Client{
		config:     cfg,
		httpClient: httpClient,
	}
}

// StreamChat 发起流式推导
func (c *Client) StreamChat(ctx context.Context, req Request) (<-chan StreamEvent, <-chan error) {
	out := make(chan StreamEvent, 64)
	errs := make(chan error, 1)

	go func() {
		// 1. 尝试 UDS / TCP 远程守护节点
		if c.config.BaseURL != "" || (c.config.SocketPath != "" && checkSocketExists(c.config.SocketPath)) {
			targetURL := "http://unix/api/v1/stream"
			if c.config.BaseURL != "" {
				targetURL = fmt.Sprintf("%s/api/v1/stream", strings.TrimRight(c.config.BaseURL, "/"))
			}

			bodyBytes, _ := json.Marshal(req)
			httpReq, err := http.NewRequestWithContext(ctx, http.MethodPost, targetURL, bytes.NewReader(bodyBytes))
			if err == nil {
				httpReq.Header.Set("Content-Type", "application/json")
				if c.config.AuthToken != "" {
					httpReq.Header.Set("Authorization", "Bearer "+c.config.AuthToken)
				}

				resp, err := c.httpClient.Do(httpReq)
				if err == nil && resp.StatusCode == http.StatusOK {
					defer resp.Body.Close()
					parseSSEStream(ctx, resp.Body, out, errs)
					return
				}
				if resp != nil {
					_ = resp.Body.Close()
				}
			}
		}

		// 2. 本地沙箱进程直调 Fallback
		if c.config.AutoFallback {
			c.streamFallback(ctx, req, out, errs)
			return
		}

		defer close(out)
		defer close(errs)
		errs <- errors.New("no available daemon backend and auto_fallback disabled")
	}()

	return out, errs
}

// Chat 单次聚合调用
func (c *Client) Chat(ctx context.Context, req Request) (*Response, error) {
	start := time.Now()
	events, errs := c.StreamChat(ctx, req)

	resp := &Response{
		SessionID: req.SessionID,
		Status:    "success",
		Model:     req.Model,
	}

	var fullContent strings.Builder
	var thinkingContent strings.Builder

	for events != nil || errs != nil {
		select {
		case ev, ok := <-events:
			if !ok {
				events = nil
				break
			}
			switch ev.Type {
			case EventThinkingDelta:
				thinkingContent.WriteString(ev.TextDelta)
			case EventContentDelta:
				fullContent.WriteString(ev.TextDelta)
			case EventDone:
				resp.Content = ev.FullContent
				if ev.ThinkingContent != nil {
					resp.ThinkingContent = ev.ThinkingContent
				}
				resp.PromptTokens = ev.PromptTokens
				resp.OutputTokens = ev.OutputTokens
			case EventError:
				resp.Status = "error"
				resp.ErrorMessage = &ev.ErrorMessage
				resp.ElapsedMs = float64(time.Since(start).Milliseconds())
				return resp, errors.New(ev.ErrorMessage)
			}
		case err, ok := <-errs:
			if !ok {
				errs = nil
				break
			}
			if err != nil {
				resp.Status = "error"
				msg := err.Error()
				resp.ErrorMessage = &msg
				resp.ElapsedMs = float64(time.Since(start).Milliseconds())
				return resp, err
			}
		}
	}

	if resp.Content == "" {
		resp.Content = fullContent.String()
	}
	if thinkingContent.Len() > 0 && resp.ThinkingContent == nil {
		t := thinkingContent.String()
		resp.ThinkingContent = &t
	}
	resp.ElapsedMs = float64(time.Since(start).Milliseconds())
	return resp, nil
}

// RunJSON 执行结构化 JSON 推导并反序列化到 target
func (c *Client) RunJSON(ctx context.Context, req Request, target interface{}) error {
	resp, err := c.Chat(ctx, req)
	if err != nil {
		return err
	}
	if resp.Status != "success" {
		return fmt.Errorf("ttagy execution failed: %v", resp.ErrorMessage)
	}

	jsonStr, err := ExtractStructuredJSON(resp.Content)
	if err != nil {
		return err
	}
	return json.Unmarshal([]byte(jsonStr), target)
}

func checkSocketExists(path string) bool {
	info, err := os.Stat(path)
	if err != nil {
		return false
	}
	return info.Mode()&os.ModeSocket != 0
}

func parseSSEStream(ctx context.Context, r io.Reader, out chan<- StreamEvent, errs chan<- error) {
	defer close(out)
	defer close(errs)

	reader := bufio.NewReaderSize(r, 64*1024)
	var dataBuf bytes.Buffer

	for {
		select {
		case <-ctx.Done():
			errs <- ctx.Err()
			return
		default:
		}

		line, isPrefix, err := reader.ReadLine()
		if err != nil {
			if err == io.EOF {
				return
			}
			select {
			case errs <- fmt.Errorf("read sse error: %w", err):
			case <-ctx.Done():
			}
			return
		}

		dataBuf.Write(line)
		if isPrefix {
			continue
		}

		fullLine := bytes.TrimSpace(dataBuf.Bytes())
		dataBuf.Reset()

		if len(fullLine) == 0 || fullLine[0] == ':' {
			continue
		}

		if bytes.HasPrefix(fullLine, []byte("data:")) {
			payload := bytes.TrimSpace(bytes.TrimPrefix(fullLine, []byte("data:")))
			if len(payload) == 0 {
				continue
			}

			var event StreamEvent
			if err := json.Unmarshal(payload, &event); err != nil {
				continue
			}

			select {
			case out <- event:
			case <-ctx.Done():
				errs <- ctx.Err()
				return
			}
		}
	}
}

func (c *Client) streamFallback(ctx context.Context, req Request, out chan<- StreamEvent, errs chan<- error) {
	defer close(out)
	defer close(errs)

	binary, err := findAgyBinary()
	if err != nil {
		errs <- err
		return
	}

	sandboxDir := filepath.Join(os.TempDir(), fmt.Sprintf("ttagy_go_%d", time.Now().UnixNano()))
	_ = os.MkdirAll(sandboxDir, 0700)
	defer os.RemoveAll(sandboxDir)

	args := []string{
		"-p", req.Prompt,
		"--output-format", "stream-json",
		"--disable-slash-commands",
		"--dangerously-skip-permissions",
		"--log-file", filepath.Join(sandboxDir, "agy.log"),
	}
	if req.Model != "" {
		args = append(args, "--model", req.Model)
	}
	if req.Effort != "" && req.Effort != "none" {
		args = append(args, "--effort", req.Effort)
	}
	if req.JSONSchema != "" {
		args = append(args, "--json-schema", req.JSONSchema)
	}

	cmd := buildGuardedCommand(ctx, binary, args, sandboxDir)

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		errs <- err
		return
	}
	stderr, err := cmd.StderrPipe()
	if err != nil {
		errs <- err
		return
	}

	stderrBuf, _ := DrainStderrAsync(stderr, 64*1024)

	if err := cmd.Start(); err != nil {
		errs <- fmt.Errorf("spawn agy failed: %w", err)
		return
	}

	out <- StreamEvent{
		Type:        EventInit,
		SessionID:   req.SessionID,
		Model:       req.Model,
		Effort:      req.Effort,
		BackendMode: "fallback_direct_spawn",
	}

	start := time.Now()
	reader := bufio.NewReader(stdout)
	for {
		line, isPrefix, err := reader.ReadLine()
		if err != nil {
			break
		}
		if isPrefix || len(line) == 0 {
			continue
		}

		items := parseNdjsonLine(line, req.SessionID, float64(time.Since(start).Milliseconds()))
		for _, item := range items {
			out <- item
		}
	}

	_ = cmd.Wait()
	if stderrLogs := stderrBuf.String(); len(stderrLogs) > 0 && ctx.Err() != nil {
		errs <- fmt.Errorf("process terminated with stderr: %s", stderrLogs)
	}
}

func findAgyBinary() (string, error) {
	if p := os.Getenv("AGY_PATH"); p != "" {
		return p, nil
	}
	return exec.LookPath("agy")
}

func parseNdjsonLine(line []byte, sessionID string, elapsed float64) []StreamEvent {
	var raw map[string]interface{}
	if err := json.Unmarshal(line, &raw); err != nil {
		return nil
	}

	var events []StreamEvent
	evType, _ := raw["type"].(string)
	if evType == "" {
		evType, _ = raw["event"].(string)
	}

	switch evType {
	case "step_update":
		if step, ok := raw["step_update"].(map[string]interface{}); ok {
			if thought, _ := step["thought_delta"].(string); thought != "" {
				events = append(events, StreamEvent{
					Type:      EventThinkingDelta,
					SessionID: sessionID,
					TextDelta: thought,
					ElapsedMs: elapsed,
				})
			}
			if text, _ := step["text_delta"].(string); text != "" {
				events = append(events, StreamEvent{
					Type:      EventContentDelta,
					SessionID: sessionID,
					TextDelta: text,
					ElapsedMs: elapsed,
				})
			}
		}
	case "result", "done":
		if res, ok := raw["result"].(map[string]interface{}); ok {
			content, _ := res["content"].(string)
			events = append(events, StreamEvent{
				Type:        EventDone,
				SessionID:   sessionID,
				FullContent: content,
				ElapsedMs:   elapsed,
			})
		}
	case "error":
		errMsg, _ := raw["error"].(string)
		events = append(events, StreamEvent{
			Type:         EventError,
			SessionID:    sessionID,
			ErrorCode:    "CLI_ERROR",
			ErrorMessage: errMsg,
			IsRetryable:  false,
		})
	}

	return events
}
