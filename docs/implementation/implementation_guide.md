# CodeNexus 实现指南

## 概述
本文档为 CodeNexus 代码库关系管理工具提供综合性的实现指南，整合了 Rust MCP 生态系统最佳实践、系统设计架构和开发规范要求，为开发团队提供完整的实现路径。

## 术语与概念定义
- **MCP (Model Context Protocol)**: 基于 JSON-RPC 的开放协议，用于 LLM 应用程序与外部数据源、工具的集成
- **rmcp**: Rust 官方 MCP SDK，提供完整的协议实现和工具支持
- **标签管理器**: 负责文件标签的增删改查和标签发现功能的核心组件
- **注释管理器**: 负责文件注释存储和获取的管理组件
- **关联管理器**: 负责文件间关联关系管理的核心组件
- **查询引擎**: 负责复杂查询逻辑处理和结果聚合的组件
- **MCP适配器**: 负责 MCP 协议实现和接口暴露的适配层

## 技术架构

### 核心技术栈
```toml
[dependencies]
rmcp = { version = "0.1", features = ["server", "transport-io"] }
rmcp-macros = "0.1"
tokio = { version = "1.0", features = ["macros", "rt-multi-thread"] }
serde = { version = "1.0", features = ["derive"] }
schemars = { version = "0.8", features = ["derive"] }
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
```

### 系统架构图
```
┌─────────────────┐    ┌─────────────────┐
│   LLM Client    │    │   CLI Client    │
└─────────────────┘    └─────────────────┘
         │                       │
         └───────────────────────┼───────────────────────┐
                                 │                       │
                    ┌─────────────────┐                  │
                    │  MCP Adapter    │                  │
                    └─────────────────┘                  │
                                 │                       │
                    ┌─────────────────┐                  │
                    │  Query Engine   │                  │
                    └─────────────────┘                  │
                                 │                       │
         ┌───────────────────────┼───────────────────────┐
         │                       │                       │
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Tag Manager    │    │Comment Manager  │    │Relation Manager │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────────┐
                    │ Data Storage    │
                    └─────────────────┘
```

## 开发环境配置

### 1. 项目结构
```
code_nexus/
├── Cargo.toml
├── src/
│   ├── main.rs              # MCP 服务器入口
│   ├── lib.rs               # 库入口
│   ├── managers/            # 核心管理器
│   │   ├── mod.rs
│   │   ├── tag_manager.rs
│   │   ├── comment_manager.rs
│   │   └── relation_manager.rs
│   ├── query/               # 查询引擎
│   │   ├── mod.rs
│   │   └── engine.rs
│   ├── mcp/                 # MCP 适配器
│   │   ├── mod.rs
│   │   └── adapter.rs
│   ├── storage/             # 数据存储
│   │   ├── mod.rs
│   │   └── json_storage.rs
│   ├── models.rs            # 数据模型
│   └── error.rs             # 错误定义
├── tests/
│   └── integration_tests.rs
└── .codenexus/              # 数据存储目录
    ├── tags.json
    ├── comments.json
    └── relations.json
```

### 2. 基础配置
```rust
// main.rs - MCP 服务器入口
use rmcp::{ServerHandler, ServiceExt, transport::stdio};
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    // 启动 MCP 服务器
    let server = CodeNexusServer::new().await?;
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

## 核心组件实现

### 1. 数据模型定义
```rust
// models.rs
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub tags: Vec<String>,
    pub comment: Option<String>,
    pub relations: Vec<Relation>,
    pub incoming_relations: Vec<Relation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub target: String,
    pub description: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TagQueryParams {
    #[schemars(description = "标签查询表达式，支持 AND、NOT、通配符")]
    pub query: String,
}
```

### 2. 标签管理器实现
```rust
// managers/tag_manager.rs
use std::collections::{HashMap, HashSet};
use anyhow::Result;

pub struct TagManager {
    file_tags: HashMap<String, HashSet<String>>,
    tag_index: HashMap<String, HashSet<String>>,
    tag_to_files: HashMap<String, HashSet<String>>,
}

impl TagManager {
    pub fn new() -> Self {
        Self {
            file_tags: HashMap::new(),
            tag_index: HashMap::new(),
            tag_to_files: HashMap::new(),
        }
    }

    pub async fn add_tags(&mut self, file_path: &str, tags: Vec<String>) -> Result<()> {
        // 验证标签格式
        for tag in &tags {
            self.validate_tag(tag)?;
        }
        
        // 更新内存索引
        let file_tags = self.file_tags.entry(file_path.to_string()).or_default();
        for tag in tags {
            file_tags.insert(tag.clone());
            self.update_indices(&tag, file_path);
        }
        
        Ok(())
    }

    fn validate_tag(&self, tag: &str) -> Result<()> {
        if !tag.contains(':') {
            anyhow::bail!("标签格式错误，应为 type:value 格式");
        }
        Ok(())
    }

    fn update_indices(&mut self, tag: &str, file_path: &str) {
        // 更新标签类型索引
        if let Some((tag_type, tag_value)) = tag.split_once(':') {
            self.tag_index.entry(tag_type.to_string())
                .or_default()
                .insert(tag_value.to_string());
        }
        
        // 更新标签到文件映射
        self.tag_to_files.entry(tag.to_string())
            .or_default()
            .insert(file_path.to_string());
    }
}
```

## MCP接口实现

### 1. MCP服务器定义
```rust
// mcp/adapter.rs
use rmcp::{ServerHandler, ServerInfo, ServerCapabilities, tool};

#[derive(Debug, Clone)]
pub struct CodeNexusServer {
    tag_manager: Arc<Mutex<TagManager>>,
    comment_manager: Arc<Mutex<CommentManager>>,
    relation_manager: Arc<Mutex<RelationManager>>,
    query_engine: Arc<QueryEngine>,
}

#[tool(tool_box)]
impl CodeNexusServer {
    #[tool(description = "为文件添加标签")]
    async fn add_file_tags(
        &self,
        #[tool(param)]
        #[schemars(description = "文件路径")]
        file_path: String,
        #[tool(param)]
        #[schemars(description = "标签列表，格式为 type:value")]
        tags: Vec<String>,
    ) -> String {
        match self.tag_manager.lock().await.add_tags(&file_path, tags).await {
            Ok(_) => "标签添加成功".to_string(),
            Err(e) => format!("错误: {}", e),
        }
    }

    #[tool(description = "根据标签查询文件")]
    async fn query_files_by_tags(
        &self,
        #[tool(aggr)] params: TagQueryParams,
    ) -> String {
        match self.query_engine.execute_tag_query(&params.query).await {
            Ok(files) => serde_json::to_string(&files).unwrap_or_default(),
            Err(e) => format!("查询错误: {}", e),
        }
    }
}

#[tool(tool_box)]
impl ServerHandler for CodeNexusServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("CodeNexus 代码库关系管理工具".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
```

## 实现步骤

### 阶段一：基础架构 (1-2周)
1. **项目初始化**
   - 创建 Rust 项目结构
   - 配置 Cargo.toml 依赖
   - 设置开发环境和工具链

2. **数据存储层**
   - 实现 JSON 文件存储
   - 设计内存数据结构
   - 实现数据加载和持久化

3. **核心管理器**
   - 实现 TagManager 基础功能
   - 实现 CommentManager 基础功能
   - 实现 RelationManager 基础功能

### 阶段二：MCP集成 (1-2周)
1. **MCP适配器开发**
   - 实现 MCP 服务器框架
   - 定义工具接口和参数验证
   - 集成核心管理器

2. **查询引擎**
   - 实现标签查询逻辑
   - 实现关联关系查询
   - 实现复合查询功能

### 阶段三：优化和测试 (1周)
1. **性能优化**
   - 内存索引优化
   - 查询性能调优
   - 异步操作优化

2. **测试和调试**
   - 单元测试编写
   - 集成测试验证
   - MCP Inspector 调试

## 测试和调试

### 1. 单元测试
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_tags() {
        let mut manager = TagManager::new();
        let result = manager.add_tags("test.rs", vec!["category:api".to_string()]).await;
        assert!(result.is_ok());
    }
}
```

### 2. MCP Inspector 调试
```bash
# 安装 MCP Inspector
npm install -g @modelcontextprotocol/inspector

# 测试服务器
npx @modelcontextprotocol/inspector cargo run
```

## 部署配置

### 1. 构建优化
```toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### 2. MCP 客户端配置
```json
{
  "mcpServers": {
    "code-nexus": {
      "command": "code-nexus",
      "args": [],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

## 性能考虑

### 1. 内存管理
- 使用 Arc<Mutex<T>> 共享状态
- 实现高效的索引结构
- 及时释放不需要的资源

### 2. 查询优化
- 内存中快速标签匹配
- 复杂查询结果缓存
- 并行数据获取

### 3. 文件操作
- 异步文件 I/O
- 批量写入优化
- 写入前数据验证

## 错误处理策略

### 1. 统一错误类型
```rust
// error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CodeNexusError {
    #[error("文件不存在: {0}")]
    FileNotFound(String),
    #[error("标签格式错误: {0}")]
    InvalidTagFormat(String),
    #[error("存储错误: {0}")]
    StorageError(#[from] std::io::Error),
}
```

### 2. 错误响应格式
- 提供详细错误信息
- 包含恢复建议
- 统一错误响应格式

## 总结
本实现指南整合了 Rust MCP 生态系统最佳实践和 CodeNexus 系统设计，提供了完整的开发路径。通过遵循本指南，开发团队可以高效地构建出高质量、高性能的代码库关系管理工具。
