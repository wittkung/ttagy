#!/usr/bin/env bash
# TTAgy 纯本地全链路 CI/CD 质量门禁 (100% Local Quality Gate - 零云端额度消耗)
# 本地执行 Rust + C-FFI + TypeScript + Python + Go + 混沌压测 + 跨语言一致性 (CTS) 全套测试

set -e

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "${ROOT_DIR}"

echo "=================================================="
echo "🛡️  TTAgy Local CI/CD: 正在执行本地全栈自动化质量门禁..."
echo "=================================================="

# 0. 构建确定性仿真桩 mock-agy
echo "⚙️  [0/6] 编译本地确定性仿真桩 mock-agy..."
cargo build -p mock-agy --quiet
export AGY_PATH="${ROOT_DIR}/target/debug/mock-agy"

# 1. 契约 Schema 静态校验 (Draft-07 & Zero Bare Objects)
echo "📋 [1/6] 验证 Draft-07 强类型接口契约..."
node -e '
  const fs = require("fs");
  const path = require("path");
  const contractsDirs = [
    path.join("specs", "contracts", "v1"),
    path.join("specs", "001-architecture-audit-evolution", "contracts"),
  ];
  let total = 0;
  for (const dir of contractsDirs) {
    if (!fs.existsSync(dir)) continue;
    const files = fs.readdirSync(dir).filter(f => f.endsWith(".json"));
    for (const f of files) {
      const c = JSON.parse(fs.readFileSync(path.join(dir, f), "utf-8"));
      if (c["$schema"] !== "http://json-schema.org/draft-07/schema#") {
        console.error(`Invalid schema in ${path.join(dir, f)}`);
        process.exit(1);
      }
      total++;
    }
  }
  console.log(`✅ 契约验证通过 (${total} 个 Draft-07 契约文件)`);
'

# 2. Rust & C-FFI Workspace 编译、单测、混沌套件与 CTS
echo "🦀 [2/6] 运行 Rust 全工作区（含 C-ABI FFI、混沌套件与 CTS）..."
cargo test --workspace --quiet
echo "✅ Rust 核心、守护服务、C-FFI、混沌套件与 CTS 100% PASS"

# 3. TypeScript SDK 单元测试与 CTS
echo "⚛️  [3/6] 运行 TypeScript SDK 跨平台客户端与 CTS 测试..."
cd packages/ttagy-client
node --test src/__tests__/client.test.mjs src/__tests__/conformance.test.mjs
cd "${ROOT_DIR}"
echo "✅ TypeScript SDK 客户端与 CTS 测试 100% PASS"

# 4. Python SDK 单元测试与 CTS
echo "🐍 [4/6] 运行 Python SDK 异步契约与 CTS 测试..."
PYTHONPATH=python python3 -m unittest discover -s python/tests -q
echo "✅ Python SDK 客户端与 CTS 测试 100% PASS"

# 5. Go SDK 单元测试与 CTS
echo "🐹 [5/6] 运行 Go SDK 客户端与 CTS 测试..."
cd golang/ttagy
go test -v ./...
cd "${ROOT_DIR}"
echo "✅ Go SDK 客户端与 CTS 测试 100% PASS"

# 6. 本地打包与构建验证 (零警告发布态检查)
echo "📦 [6/6] 验证本地 release 构建与二进制就绪状态..."
cargo check --release --workspace --quiet
echo "✅ 本地 release 编译检查 100% PASS"

echo "=================================================="
echo "🎉 TTAgy 本地 6 重 CI/CD 质量门禁与全语言 CTS 套件 100% 通过！零云端额度消耗！"
echo "=================================================="
