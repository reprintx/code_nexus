# Rust MCP 生态系统指南

## 概述
本文档总结了 Rust 语言在 Model Context Protocol (MCP) 生态系统中的现状、主要工具库、最佳实践和开发指南。

## MCP 简介
Model Context Protocol (MCP) 是一个开放协议，用于在 LLM 应用程序和外部数据源、工具之间实现无缝集成。它基于 JSON-RPC 协议，提供了标准化的通信方式。

## Rust MCP 生态现状

### 官方 SDK
**rmcp (Rust Model Context Protocol)**
- **仓库**: https://github.com/modelcontextprotocol/rust-sdk
- **Crate**: `rmcp` (https://crates.io/crates/rmcp)
- **状态**: 官方维护，活跃开发中
- **版本**: 0.1.x (等待首个正式版本)
- **特性**: 完整的 MCP 协议实现，支持服务器和客户端

### 核心特性
- **异步支持**: 基于 tokio 异步运行时
- **宏支持**: 提供 `#[tool]` 宏简化工具定义
- **传输层**: 支持 stdio、TCP 等多种传输方式
- **类型安全**: 完整的 Rust 类型系统支持
- **JSON Schema**: 集成 schemars 自动生成参数模式

## 主要组件

### 1. rmcp 核心库
```toml
[dependencies]
rmcp = { version = "0.1", features = ["server", "transport-io"] }
```

**主要功能**:
- MCP 协议实现
- 服务器和客户端支持
- 工具定义和管理
- 传输层抽象

### 2. rmcp-macros
```toml
[dependencies]
rmcp-macros = "0.1"
```

**主要功能**:
- `#[tool]` 宏用于定义 MCP 工具
- `#[tool(tool_box)]` 宏用于工具集合
- 自动生成 JSON Schema
- 简化样板代码

### 3. 必需依赖
```toml
[dependencies]
tokio = { version = "1.0", features = ["macros", "rt-multi-thread"] }
serde = { version = "1.0", features = ["derive"] }
schemars = { version = "0.8", features = ["derive"] }
anyhow = "1.0"
tracing = "0.1"
```

## 开发模式

### 1. 服务器开发模式
```rust
use rmcp::{ServerHandler, ServiceExt, tool, transport::stdio};

#[derive(Debug, Clone)]
pub struct MyServer {
    // 服务器状态
}

#[tool(tool_box)]
impl MyServer {
    #[tool(description = "工具描述")]
    async fn my_tool(
        &self,
        #[tool(param)]
        #[schemars(description = "参数描述")]
        param: String,
    ) -> String {
        // 工具实现
        format!("Result: {}", param)
    }
}

#[tool(tool_box)]
impl ServerHandler for MyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("服务器说明".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let service = MyServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

### 2. 客户端开发模式
```rust
use rmcp::{ServiceExt, transport::TokioChildProcess};
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<()> {
    let client = ().serve(
        TokioChildProcess::new(Command::new("my-mcp-server"))?
    ).await?;
    
    // 使用客户端调用工具
    let result = client.call_tool("my_tool", params).await?;
    Ok(())
}
```

## 最佳实践

### 1. 项目结构
```
my_mcp_server/
├── Cargo.toml
├── src/
│   ├── main.rs          # MCP 服务器入口
│   ├── lib.rs           # 库入口
│   ├── tools/           # 工具实现
│   │   ├── mod.rs
│   │   └── my_tool.rs
│   ├── models.rs        # 数据模型
│   └── error.rs         # 错误定义
└── examples/
    └── usage.rs
```

### 2. 错误处理
```rust
use anyhow::{Result, Context};

#[tool(description = "示例工具")]
async fn example_tool(&self, input: String) -> String {
    match self.process_input(&input).await {
        Ok(result) => serde_json::to_string(&result).unwrap_or_default(),
        Err(e) => format!("Error: {}", e),
    }
}
```

### 3. 日志记录
```rust
use tracing::{info, error, debug};
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    info!("Starting MCP server");
    // ...
}
```

### 4. 参数验证
```rust
use schemars::JsonSchema;

#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct ToolParams {
    #[schemars(description = "必需参数")]
    required_field: String,
    #[schemars(description = "可选参数")]
    optional_field: Option<i32>,
}

#[tool(description = "带参数验证的工具")]
async fn validated_tool(
    &self,
    #[tool(aggr)] params: ToolParams,
) -> String {
    // 参数已自动验证和反序列化
    format!("Processing: {}", params.required_field)
}
```

## 测试和调试

### 1. MCP Inspector
```bash
# 安装 MCP Inspector
npm install -g @modelcontextprotocol/inspector

# 测试服务器
npx @modelcontextprotocol/inspector cargo run
```

### 2. 单元测试
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_my_tool() {
        let server = MyServer::new();
        let result = server.my_tool("test".to_string()).await;
        assert_eq!(result, "Result: test");
    }
}
```

## 社区项目示例

### 1. 天气服务器
- **功能**: 提供天气数据查询
- **API**: National Weather Service API
- **特点**: 多步骤 API 调用，数据格式化

### 2. 文件系统服务器
- **功能**: 文件系统访问
- **特点**: 安全的文件操作，权限控制

### 3. 数据库服务器
- **功能**: 数据库查询和操作
- **特点**: SQL 查询，结果格式化

## 部署和分发

### 1. 二进制分发
```toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
```

### 2. 容器化
```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/my-mcp-server /usr/local/bin/
CMD ["my-mcp-server"]
```

### 3. 配置文件
```json
{
  "mcpServers": {
    "my-server": {
      "command": "my-mcp-server",
      "args": []
    }
  }
}
```

## 性能考虑

### 1. 异步处理
- 使用 tokio 异步运行时
- 避免阻塞操作
- 合理使用并发

### 2. 内存管理
- 使用 Arc 和 Mutex 共享状态
- 避免不必要的克隆
- 及时释放资源

### 3. 错误处理
- 使用 Result 类型
- 提供有意义的错误信息
- 避免 panic

## 未来发展

### 1. 生态系统成熟度
- 官方 SDK 趋于稳定
- 社区项目增多
- 最佳实践形成

### 2. 功能扩展
- 更多传输层支持
- 增强的工具定义
- 性能优化

### 3. 工具链完善
- 更好的调试工具
- 自动化测试框架
- 部署工具

## 学习资源

### 1. 官方文档
- MCP 规范: https://spec.modelcontextprotocol.io/
- Rust SDK: https://github.com/modelcontextprotocol/rust-sdk

### 2. 示例项目
- 官方示例: https://github.com/modelcontextprotocol/rust-sdk/tree/main/examples
- 社区项目: GitHub 搜索 "mcp rust"

### 3. 社区资源
- Discord 社区
- GitHub Discussions
- 技术博客和教程

## 总结
Rust MCP 生态系统正在快速发展，官方 `rmcp` crate 提供了强大的基础设施。通过遵循最佳实践和使用合适的工具，可以高效地开发出高质量的 MCP 服务器和客户端。
