# Quickstart: Omni-Language SDK Ecosystem

**Feature**: [`specs/003-omni-language-sdk-ecosystem`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/003-omni-language-sdk-ecosystem/spec.md)
**Status**: `Ready for Verification`
**Created**: 2026-08-24

---

## 1. C / C++ Usage

```c
#include "ttagy.h"
#include <stdio.h>

int main() {
    ttagy_client_t* client = ttagy_client_new(NULL, "/tmp/ttagy.sock", NULL, true);
    char* response = NULL;
    int32_t rc = ttagy_chat_sync(client, "Hello from C", "gemini-3.7-flash", &response);
    if (rc == 0 && response != NULL) {
        printf("Response: %s\n", response);
        ttagy_string_free(response);
    }
    ttagy_client_free(client);
    return 0;
}
```

---

## 2. Go Usage

```go
package main

import (
    "context"
    "fmt"
    "github.com/wittkung/ttagy/golang/ttagy"
)

func main() {
    client := ttagy.NewClient(ttagy.ClientConfig{
        SocketPath:   "/tmp/ttagy.sock",
        AutoFallback: true,
    })

    resp, err := client.Chat(context.Background(), ttagy.Request{
        Prompt: "Hello from Go",
        Model:  "gemini-3.7-flash",
    })
    if err == nil {
        fmt.Println("Response:", resp.Content)
    }
}
```

---

## 3. Dart Usage

```dart
import 'package:ttagy/ttagy.dart';

void main() async {
  final client = TtagyClient(autoFallback: true);
  final response = await client.chat(TtagyRequest(
    prompt: 'Hello from Dart',
    model: 'gemini-3.7-flash',
  ));
  print('Response: ${response.content}');
}
```
