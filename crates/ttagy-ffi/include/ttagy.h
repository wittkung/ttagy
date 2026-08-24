/**
 * TTAgy C-ABI Native FFI Header
 *
 * Safe C/C++ & Swift 6 Header for TTAgy Native Bridge
 */

#ifndef TTAGY_H
#define TTAGY_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#if defined(__has_feature) && __has_feature(nullability)
#define TTAGY_NONNULL _Nonnull
#define TTAGY_NULLABLE _Nullable
#else
#define TTAGY_NONNULL
#define TTAGY_NULLABLE
#endif

#ifdef __cplusplus
#define TTAGY_EXTERN_C extern "C"
#else
#define TTAGY_EXTERN_C extern
#endif

#if defined(__has_attribute) && __has_attribute(swift_name)
#define TTAGY_SWIFT_NAME(name) __attribute__((swift_name(name)))
#else
#define TTAGY_SWIFT_NAME(name)
#endif

#if defined(__has_attribute) && __has_attribute(enum_extensibility)
#define TTAGY_CLOSED_ENUM __attribute__((enum_extensibility(closed)))
#else
#define TTAGY_CLOSED_ENUM
#endif

#ifdef __cplusplus
extern "C" {
#endif

/// 错误码定义
typedef enum TTAGY_CLOSED_ENUM TtagyErrorCode {
    TTAGY_OK = 0,
    TTAGY_ERR_INVALID_ARGUMENT = -1,
    TTAGY_ERR_NULL_POINTER = -2,
    TTAGY_ERR_INIT_FAILED = -3,
    TTAGY_ERR_REQUEST_FAILED = -4,
    TTAGY_ERR_SERIALIZATION = -5,
    TTAGY_ERR_PANIC = -999,
} TtagyErrorCode;

/// TtagyClient 不透明结构体句柄
typedef struct ttagy_client_t ttagy_client_t;

/// 创建 TTAGY 客户端实例
/// @param config_json JSON 格式的配置字符串（可选，为 NULL 时采用默认配置）
/// @param out_client 输出的客户端句柄指针
/// @return 0 成功，负数表示错误码
TTAGY_EXTERN_C int32_t ttagy_client_create(
    const char * TTAGY_NULLABLE config_json,
    ttagy_client_t * TTAGY_NULLABLE * TTAGY_NONNULL out_client
) TTAGY_SWIFT_NAME(TtagyClient.create(configJSON:outClient:));

/// 释放 TTAGY 客户端实例
/// @param client 客户端句柄
TTAGY_EXTERN_C void ttagy_client_free(
    ttagy_client_t * TTAGY_NULLABLE client
) TTAGY_SWIFT_NAME(ttagy_client_free(client:));

/// 同步阻塞执行单轮推导
/// @param client 客户端句柄
/// @param request_json JSON 格式的 TtagyRequest 字符串
/// @param out_response_json 输出由 Rust 分配的 JSON 字符串指针 (必须调用 ttagy_string_free 释放)
/// @return 0 成功，负数表示错误码
TTAGY_EXTERN_C int32_t ttagy_client_chat(
    ttagy_client_t * TTAGY_NONNULL client,
    const char * TTAGY_NONNULL request_json,
    char * TTAGY_NULLABLE * TTAGY_NONNULL out_response_json
) TTAGY_SWIFT_NAME(ttagy_client_chat(client:requestJSON:outResponseJSON:));

/// 释放由 Rust 分配导出的 UTF-8 C 字符串
/// @param str 待释放的字符串指针
TTAGY_EXTERN_C void ttagy_string_free(
    char * TTAGY_NULLABLE str
) TTAGY_SWIFT_NAME(ttagy_string_free(str:));

/// 获取当前调用线程最后一次发生的错误描述信息 (只读，调用方无需释放)
/// @return UTF-8 编码的错误字符串指针，若无错误则返回空字符串
TTAGY_EXTERN_C const char * TTAGY_NONNULL ttagy_last_error_message(void)
TTAGY_SWIFT_NAME(ttagy_last_error_message());

#ifdef __cplusplus
}
#endif

#endif /* TTAGY_H */
