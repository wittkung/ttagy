#include "ttagy.h"
#include <stdio.h>
#include <stdlib.h>

int main() {
    printf("⚡ Connecting to TTAgy via C-ABI Native FFI...\n");

    ttagy_client_t *client = ttagy_client_create();
    if (!client) {
        fprintf(stderr, "Failed to create TTAgy client\n");
        return 1;
    }

    ttagy_response_t *resp = NULL;
    int32_t ret = ttagy_client_chat(client, "What is zero-cost abstraction in systems programming?", &resp);

    if (ret == 0 && resp != NULL) {
        printf("✅ Success!\nStatus: %s\nElapsed: %.2fms\nContent:\n%s\n",
               resp->status, resp->elapsed_ms, resp->content);
        ttagy_response_free(resp);
    } else {
        char err_buf[256] = {0};
        ttagy_last_error_message(err_buf, sizeof(err_buf));
        fprintf(stderr, "❌ Error (%d): %s\n", ret, err_buf);
    }

    ttagy_client_free(client);
    return 0;
}
