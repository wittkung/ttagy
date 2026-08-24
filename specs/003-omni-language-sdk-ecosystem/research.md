# Research: Omni-Language SDK Ecosystem Architecture & Safety Standards

**Feature**: [`specs/003-omni-language-sdk-ecosystem`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/003-omni-language-sdk-ecosystem/spec.md)
**Status**: `Completed & Audited by Subagents`
**Created**: 2026-08-24

---

## 1. C-ABI Native FFI & Swift Interoperability (Audited by Subagent 1)

### 1.1 Panic Barrier & Safety Macro (`ffi_guard!`)
- **Danger**: Panicking across `extern "C"` boundaries causes undefined behavior (UB), stack frame corruption, or instant process abort.
- **Decision**: 100% of exported `extern "C"` functions are wrapped with `ffi_guard!` using `std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ...))`. Panics are caught, converted to `TTAGY_ERR_PANIC (-999)`, and recorded in thread-local storage (`ttagy_last_error_message()`).

### 1.2 Opaque Pointer & Runtime Context
- **Design**: C header exposes `typedef struct ttagy_client_t ttagy_client_t;`. Rust backend boxes `TtagyClientContext` (holding `Arc<TtagyClient>` and a dedicated Tokio `Runtime`), returning `*mut ttagy_client_t` via `Box::into_raw`.
- **Memory Boundary**: Strict Callee-allocates / Callee-frees rule. Every Rust-allocated string pointer returned to C must be deallocated using `ttagy_string_free(char*)`, and every client handle via `ttagy_client_free(ttagy_client_t*)`.

### 1.3 Clang & Swift 6 Native Support
- Header uses Clang `_Nonnull` / `_Nullable` annotations, `__attribute__((swift_name("...")))`, and `__attribute__((enum_extensibility(closed)))`.
- Includes `module.modulemap` allowing Swift code to `import TtagyC` and invoke `TtagyClient` as a typed, memory-safe Swift 6 `@unchecked Sendable` async actor.

---

## 2. Go Native SDK Architecture (Audited by Subagent 2)

### 2.1 100% Standard Library Zero-Dependency Design
- **UDS Transport**: Implements custom `http.Transport.DialContext` using `net.Dialer.DialContext(ctx, "unix", socketPath)`.
- **SSE Stream Reader**: Uses `bufio.Reader` instead of `bufio.Scanner` to avoid the 64KB token buffer overflow limitation.
- **Process Lifecycle**: Direct spawn fallback sets `cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}` and `cmd.Cancel = func() error { return syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL) }` to prevent orphan process leaks.
- **Non-blocking Stderr Drainer**: Uses `RollingBuffer` (64KB bounded) in a separate goroutine to prevent pipe deadlocks.

---

## 3. Dart / Flutter SDK Architecture (Audited by Subagent 2)

### 3.1 100% `dart:io` Native UDS & Reactive Streams
- **UDS HTTP Client**: Configures `HttpClient.connectionFactory` using `Socket.startConnect(InternetAddress(socketPath, type: InternetAddressType.unix), 0)`.
- **Reactive Streaming**: Uses `StreamController<TtagyStreamEvent>` with `onCancel` hook to abort HTTP requests or kill subprocesses on UI unmount.
- **Balanced-Brace JSON State Machine**: Lexical parser tracking `inString`, `escape`, and `{}` / `[]` depth.

---

## 4. JVM (Java 17+ / Kotlin) & .NET (C# .NET 8+) Architecture (Audited by Subagent 3)

### 4.1 JVM Platform
- Java 16+ JEP 380 `UnixDomainSocketAddress` with `SocketChannel` for zero-dependency UDS HTTP framing; `HttpClient` for TCP HTTP/SSE.
- Java `CompletableFuture<Stream<TtagyStreamEvent>>` and Kotlin `Flow<TtagyStreamEvent>`.

### 4.2 .NET Platform
- .NET 8 `SocketsHttpHandler.ConnectCallback` with `UnixDomainSocketEndPoint`.
- `IAsyncEnumerable<TtagyStreamEvent>` with `[EnumeratorCancellation]` and `CancellationToken`.
