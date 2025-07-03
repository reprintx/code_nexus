# CodeNexus

CodeNexus 是一个基于 Rust 和 Model Context Protocol (MCP) 的代码库关系管理工具，通过标签、注释和关联关系帮助开发者更好地组织和理解代码结构。

## 功能特性

- **标签管理**: 为文件添加结构化标签 (type:value 格式)
- **注释系统**: 为文件添加描述性注释
- **关联关系**: 建立文件间的依赖和关联关系
- **智能查询**: 支持复杂的标签查询和关系搜索
- **多项目支持**: 同时管理多个项目，每个项目独立存储
- **路径验证**: 确保文件路径安全性和有效性
- **MCP 集成**: 通过 MCP 协议与 AI 助手无缝集成

## 快速开始

### 安装

```bash
# 克隆项目
git clone <repository-url>
cd code_nexus

# 构建项目
cargo build --release
```

### 运行

```bash
# 启动 MCP 服务器
cargo run
```

### 配置 MCP 客户端

在你的 MCP 客户端配置中添加：

```json
{
  "mcpServers": {
    "code-nexus": {
      "command": "path/to/code-nexus",
      "args": [],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

## 使用示例

### 标签管理

```bash
# 为文件添加标签（需要提供项目路径）
add_file_tags({
  "project_path": "/path/to/your/project",
  "file_path": "src/api/user.rs",
  "tags": ["category:api", "status:active", "tech:rust"]
})

# 查询带有特定标签的文件
query_files_by_tags({
  "project_path": "/path/to/your/project",
  "query": "category:api AND status:active"
})

# 获取所有标签
get_all_tags({
  "project_path": "/path/to/your/project"
})
```

### 注释管理

```bash
# 添加文件注释
add_file_comment({
  "project_path": "/path/to/your/project",
  "file_path": "src/api/user.rs",
  "comment": "用户管理API，包含登录、注册等功能"
})

# 更新注释
update_file_comment({
  "project_path": "/path/to/your/project",
  "file_path": "src/api/user.rs",
  "comment": "用户管理API，支持OAuth登录"
})
```

### 关联关系

```bash
# 添加文件关联
add_file_relation({
  "project_path": "/path/to/your/project",
  "from_file": "src/api/user.rs",
  "to_file": "src/models/user.rs",
  "description": "依赖用户数据模型"
})

# 查询文件关联
query_file_relations({
  "project_path": "/path/to/your/project",
  "file_path": "src/api/user.rs"
})
```

## 多项目支持

CodeNexus 支持同时管理多个项目，每个项目都有独立的数据存储：

### 项目隔离
- 每个项目在其根目录下创建 `.codenexus/` 数据目录
- 项目间的标签、注释、关联关系完全隔离
- 支持同时操作多个项目而不会相互干扰

### 路径安全
- 自动验证项目路径和文件路径的有效性
- 防止路径遍历攻击，确保文件操作在项目范围内
- 支持相对路径和绝对路径

### 使用示例

```bash
# 项目A的操作
add_file_tags({
  "project_path": "/path/to/project-a",
  "file_path": "src/main.rs",
  "tags": ["category:entry", "lang:rust"]
})

# 项目B的操作（完全独立）
add_file_tags({
  "project_path": "/path/to/project-b",
  "file_path": "src/main.rs",
  "tags": ["category:api", "lang:rust"]
})
```

## 项目结构

```
code_nexus/
├── src/
│   ├── main.rs              # 程序入口
│   ├── lib.rs               # 库入口
│   ├── managers/            # 核心管理器
│   │   ├── tag_manager.rs   # 标签管理
│   │   ├── comment_manager.rs # 注释管理
│   │   └── relation_manager.rs # 关联关系管理
│   ├── query/               # 查询引擎
│   ├── mcp/                 # MCP 适配器
│   ├── storage/             # 数据存储
│   ├── models.rs            # 数据模型
│   └── error.rs             # 错误处理
├── tests/                   # 测试文件
├── docs/                    # 文档
└── .codenexus/              # 数据存储目录
    ├── tags.json            # 标签数据
    ├── comments.json        # 注释数据
    └── relations.json       # 关联关系数据
```

## 开发

### 运行测试

```bash
cargo test
```

### 代码检查

```bash
cargo check
cargo clippy
```

### 格式化代码

```bash
cargo fmt
```

## 技术栈

- **Rust**: 核心编程语言
- **rmcp**: Rust MCP SDK
- **tokio**: 异步运行时
- **serde**: 序列化/反序列化
- **tracing**: 日志记录
- **anyhow/thiserror**: 错误处理

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

MIT License
