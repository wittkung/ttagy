# Specification: Omni-Language SDK Ecosystem for TTAgy

**Classification**: `[Full SDD]`
**Status**: `Draft`
**Created**: 2026-08-24
**Feature Directory**: `specs/003-omni-language-sdk-ecosystem`

---

## 1. Executive Summary & Problem Statement

TTAgy serves as the shared local AI bridge across diverse desktop, backend, and embedded host applications. While Rust, TypeScript, and Python SDKs are fully established, native applications in **C/C++, Go, Dart/Flutter, Java/Kotlin, and C#/.NET** currently lack official, idiomatic, and contract-aligned SDKs.

This specification defines the design and functional requirements for the Omni-Language SDK Ecosystem of TTAgy:
1. **C-ABI Super-Bridge (`crates/ttagy-ffi` & `ttagy.h`)**: Native shared library exposing a robust C-ABI with thread-safety, defensive memory boundaries, and callback/polling stream drivers for C, C++, Swift, and low-level bindings.
2. **Go Native SDK (`golang/ttagy`)**: Zero-dependency Go package utilizing `net/http`, Unix domain sockets, and channel-based streaming with `context.Context` cancellation.
3. **Dart / Flutter SDK (`dart/ttagy`)**: Reactive `Stream<TtagyStreamEvent>` SDK supporting Flutter mobile/desktop apps with UDS and SSE streaming.
4. **Java / Kotlin SDK (`jvm/ttagy`)**: Modern Java 17+ `HttpClient` and Kotlin Coroutines `Flow<TtagyStreamEvent>` SDK.
5. **C# / .NET SDK (`dotnet/ttagy`)**: .NET 8+ SDK utilizing `IAsyncEnumerable<TtagyStreamEvent>` and `CancellationToken`.
6. **CTS Conformance Integration**: All SDKs are validated against `mock-agy` using the shared JSON conformance fixtures in `tests/conformance/fixtures/`.

---

## 2. User Scenarios & Personas

### Persona 1: C / C++ & Swift Native App Developer (e.g. TTZip, macOS / Linux UI)
- **Goal**: Embed TTAgy directly into a native C++ or Swift desktop application without launching separate Node.js or Python runtimes.
- **Flow**: Links with `libttagy.dylib` or `#include "ttagy.h"`; creates client; initiates stream with a C callback; receives structured events.

### Persona 2: Go Backend & Microservice Developer
- **Goal**: Integrate TTAgy local LLM bridging into a Go CLI or backend daemon with zero external dependencies.
- **Flow**: Imports `github.com/wittkung/ttagy/golang/ttagy`; initiates streaming over `/tmp/ttagy.sock` with `context.WithTimeout`; receives events via `<-chan TtagyStreamEvent`.

### Persona 3: Flutter / Dart Cross-Platform Developer
- **Goal**: Build a cross-platform Flutter app consuming stream reasoning from local Antigravity nodes.
- **Flow**: Adds `ttagy` Dart package; listens to `client.streamChat(...)` with `StreamBuilder` or async `await for`.

### Persona 4: JVM & .NET Enterprise Developer
- **Goal**: Call local AI infrastructure from Kotlin or C# enterprise microservices.
- **Flow**: Consumes `client.streamChat(req)` as a Kotlin `Flow` or C# `IAsyncEnumerable`.

---

## 3. Scope & System Boundaries

### In Scope
- `crates/ttagy-ffi`: Rust C-ABI export library producing `libttagy.{dylib,so,a}` and `include/ttagy.h`.
- `golang/ttagy`: Go module supporting UDS, TCP SSE, local fallback, streaming channels, and JSON extraction.
- `dart/ttagy`: Dart package supporting UDS, TCP SSE, local fallback, `Stream<TtagyStreamEvent>`, and JSON extraction.
- `jvm/ttagy`: Java / Kotlin SDK with `CompletableFuture` and `Flow`.
- `dotnet/ttagy`: .NET SDK with `IAsyncEnumerable`.
- Full CTS validation across all newly introduced SDKs.

### Out of Scope
- Direct binary distribution to public package registries (crates.io, npm, PyPI, pub.dev, Maven Central, NuGet) in this initial phase; repository-local source and workspace packaging is the primary target.

---

## 4. Functional Requirements

### FR-01: C-ABI Super-Bridge (`crates/ttagy-ffi`)
- **FR-01.1**: The library MUST expose standard C functions in `include/ttagy.h`:
  - `ttagy_client_t* ttagy_client_new(const char* base_url, const char* socket_path, const char* auth_token, bool auto_fallback);`
  - `void ttagy_client_free(ttagy_client_t* client);`
  - `int32_t ttagy_chat_sync(ttagy_client_t* client, const char* prompt, const char* model, char** out_response);`
  - `void ttagy_string_free(char* s);`
- **FR-01.2**: The FFI MUST guarantee memory safety, zero heap corruption, null pointer guards, and panic safety (`std::panic::catch_unwind`).

### FR-02: Go Native SDK (`golang/ttagy`)
- **FR-02.1**: The Go SDK MUST be 100% standard library (zero external third-party dependencies).
- **FR-02.2**: The Go SDK MUST support:
  - `StreamChat(ctx context.Context, req TtagyRequest) (<-chan TtagyStreamEvent, <-chan error)`
  - `Chat(ctx context.Context, req TtagyRequest) (*TtagyResponse, error)`
  - `RunJSON(ctx context.Context, req TtagyRequest, target interface{}) error`
- **FR-02.3**: The Go SDK MUST support Unix Domain Socket dialer (`/tmp/ttagy.sock`), TCP HTTP/SSE, and in-process fallback direct spawn with non-blocking stderr drain.

### FR-03: Dart / Flutter SDK (`dart/ttagy`)
- **FR-03.1**: The Dart SDK MUST provide:
  - `Stream<TtagyStreamEvent> streamChat(TtagyRequest request)`
  - `Future<TtagyResponse> chat(TtagyRequest request)`
  - `Future<T> runJson<T>(TtagyRequest request)`
- **FR-03.2**: The Dart SDK MUST support syntax-aware balanced-brace JSON extraction.

### FR-04: Java & Kotlin SDK (`jvm/ttagy`)
- **FR-04.1**: The JVM SDK MUST target Java 17+ and provide typed records/classes for `TtagyRequest`, `TtagyResponse`, `TtagyStreamEvent`.
- **FR-04.2**: The JVM SDK MUST support asynchronous reactive streaming over HTTP/SSE.

### FR-05: C# / .NET SDK (`dotnet/ttagy`)
- **FR-05.1**: The .NET SDK MUST target .NET 8+ and provide:
  - `IAsyncEnumerable<TtagyStreamEvent> StreamChatAsync(TtagyRequest request, CancellationToken cancellationToken)`
  - `Task<TtagyResponse> ChatAsync(TtagyRequest request, CancellationToken cancellationToken)`
  - `Task<T> RunJsonAsync<T>(TtagyRequest request, CancellationToken cancellationToken)`

### FR-06: Cross-Language Conformance Suite (CTS) Expansion
- **FR-06.1**: Every added SDK MUST implement automated conformance tests against `mock-agy` using `tests/conformance/fixtures/*.json`.
- **FR-06.2**: All CTS suites MUST be executed and pass in `scripts/local-ci.sh`.

---

## 5. Non-Functional Requirements & Success Criteria

| Metric | Target | Verification Method |
| :--- | :--- | :--- |
| **Language Coverage** | 8 Top Languages (Rust, TS, Python, C, C++, Go, Dart, Java/Kotlin, C#) | Existence of idiomatic SDKs with automated tests |
| **Contract Invariance** | 100% CTS pass across all SDKs | CTS runner in CI |
| **Cloud Quota & Cost** | 0 tokens ($0.00) consumed | CI execution using `mock-agy` |
| **Build & Test Speed** | $\le 10.0\text{s}$ total CI run | `time bash scripts/local-ci.sh` |
