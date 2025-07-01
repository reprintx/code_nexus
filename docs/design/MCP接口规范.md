# CodeNexus MCP 接口规范

## 1. MCP 协议概述

CodeNexus 基于 Model Context Protocol (MCP) 实现，提供标准化的接口供 AI 助手访问代码库元数据。MCP 协议采用 JSON-RPC 2.0 标准，支持双向通信和实时数据同步。

### 1.1 服务器能力声明

```json
{
  "capabilities": {
    "resources": {
      "subscribe": true,
      "listChanged": true
    },
    "tools": {
      "listChanged": true
    },
    "prompts": {
      "listChanged": true
    }
  },
  "serverInfo": {
    "name": "CodeNexus",
    "version": "1.0.0",
    "description": "代码库关系管理工具"
  }
}
```

### 1.2 协议特性

- **资源订阅**：支持实时监听文件元数据变更
- **工具调用**：提供丰富的元数据操作工具
- **提示模板**：预定义的智能分析提示
- **错误处理**：标准化的错误响应格式

## 2. Resources（资源）接口

### 2.1 文件元数据资源

**资源标识符**：`codenexus://file/{file_path}`

**资源描述**：
```json
{
  "uri": "codenexus://file/src/main.rs",
  "name": "File Metadata: src/main.rs",
  "description": "Complete metadata for file including tags, comments, and relationships",
  "mimeType": "application/json"
}
```

**资源内容示例**：
```json
{
  "file_path": "src/main.rs",
  "hash": "sha256:abc123...",
  "tags": ["backend", "core"],
  "comment": "主入口文件，包含应用启动逻辑",
  "relationships": [
    {
      "target": "src/lib.rs",
      "type": "imports", 
      "description": "导入核心库函数"
    }
  ],
  "updated_at": "2025-07-01T15:30:00Z"
}
```

### 2.2 标签列表资源

**资源标识符**：`codenexus://tags`

**资源描述**：
```json
{
  "uri": "codenexus://tags",
  "name": "All Tags",
  "description": "List of all available tags with their descriptions and usage statistics",
  "mimeType": "application/json"
}
```

**资源内容示例**：
```json
{
  "tags": [
    {
      "name": "backend",
      "description": "后端相关文件",
      "color": "#ff6b6b",
      "file_count": 15,
      "created_at": "2025-07-01T10:00:00Z"
    },
    {
      "name": "core",
      "description": "核心功能文件",
      "color": "#4ecdc4", 
      "file_count": 8,
      "created_at": "2025-07-01T10:00:00Z"
    }
  ],
  "total_count": 2
}
```

### 2.3 关系图资源

**资源标识符**：`codenexus://relationships/{file_path}`

**资源描述**：
```json
{
  "uri": "codenexus://relationships/src/main.rs",
  "name": "File Relationships: src/main.rs",
  "description": "All relationships for the specified file including incoming and outgoing connections",
  "mimeType": "application/json"
}
```

**资源内容示例**：
```json
{
  "file_path": "src/main.rs",
  "outgoing_relationships": [
    {
      "target": "src/lib.rs",
      "type": "imports",
      "description": "导入核心库函数"
    }
  ],
  "incoming_relationships": [
    {
      "source": "src/bin/cli.rs",
      "type": "calls",
      "description": "CLI工具调用主函数"
    }
  ],
  "relationship_count": {
    "outgoing": 1,
    "incoming": 1,
    "total": 2
  }
}
```

## 3. Tools（工具）接口

### 3.1 添加标签工具

**工具名称**：`add_tag`

**工具定义**：
```json
{
  "name": "add_tag",
  "description": "Add one or more tags to a file",
  "inputSchema": {
    "type": "object",
    "properties": {
      "file_path": {
        "type": "string",
        "description": "Path to the file relative to project root"
      },
      "tags": {
        "type": "array",
        "items": {"type": "string"},
        "description": "List of tags to add",
        "minItems": 1
      }
    },
    "required": ["file_path", "tags"]
  }
}
```

**响应示例**：
```json
{
  "success": true,
  "message": "Successfully added 2 tags to src/main.rs",
  "added_tags": ["backend", "core"],
  "total_tags": 3,
  "updated_at": "2025-07-01T15:30:00Z"
}
```

### 3.2 添加注释工具

**工具名称**：`add_comment`

**工具定义**：
```json
{
  "name": "add_comment",
  "description": "Add or update comment for a file",
  "inputSchema": {
    "type": "object", 
    "properties": {
      "file_path": {
        "type": "string",
        "description": "Path to the file relative to project root"
      },
      "comment": {
        "type": "string",
        "description": "Comment text (supports Markdown)",
        "maxLength": 2000
      }
    },
    "required": ["file_path", "comment"]
  }
}
```

**响应示例**：
```json
{
  "success": true,
  "message": "Successfully updated comment for src/main.rs",
  "comment_length": 45,
  "updated_at": "2025-07-01T15:30:00Z"
}
```

### 3.3 创建关系工具

**工具名称**：`create_relationship`

**工具定义**：
```json
{
  "name": "create_relationship",
  "description": "Create a relationship between two files",
  "inputSchema": {
    "type": "object",
    "properties": {
      "source_path": {
        "type": "string",
        "description": "Source file path"
      },
      "target_path": {
        "type": "string", 
        "description": "Target file path"
      },
      "relationship_type": {
        "type": "string",
        "enum": ["imports", "calls", "configures", "depends_on"],
        "description": "Type of relationship"
      },
      "description": {
        "type": "string",
        "description": "Optional description of the relationship",
        "maxLength": 500
      }
    },
    "required": ["source_path", "target_path", "relationship_type"]
  }
}
```

**响应示例**：
```json
{
  "success": true,
  "message": "Successfully created relationship between src/main.rs and src/lib.rs",
  "relationship": {
    "source": "src/main.rs",
    "target": "src/lib.rs",
    "type": "imports",
    "description": "导入核心库函数"
  },
  "created_at": "2025-07-01T15:30:00Z"
}
```

### 3.4 查询文件工具

**工具名称**：`query_files`

**工具定义**：
```json
{
  "name": "query_files",
  "description": "Query files by tags, comments, or relationships",
  "inputSchema": {
    "type": "object",
    "properties": {
      "tags": {
        "type": "array",
        "items": {"type": "string"},
        "description": "Filter by tags (AND logic)"
      },
      "comment_contains": {
        "type": "string",
        "description": "Filter by comment content (full-text search)"
      },
      "related_to": {
        "type": "string",
        "description": "Find files related to this file"
      },
      "relationship_type": {
        "type": "string",
        "enum": ["imports", "calls", "configures", "depends_on"],
        "description": "Filter by relationship type"
      },
      "limit": {
        "type": "integer",
        "minimum": 1,
        "maximum": 100,
        "default": 20,
        "description": "Maximum number of results"
      }
    }
  }
}
```

**响应示例**：
```json
{
  "success": true,
  "results": [
    {
      "file_path": "src/main.rs",
      "tags": ["backend", "core"],
      "comment": "主入口文件，包含应用启动逻辑",
      "match_reason": "tags: backend, core"
    },
    {
      "file_path": "src/lib.rs", 
      "tags": ["backend", "library"],
      "comment": "核心库文件，提供主要功能",
      "match_reason": "tags: backend"
    }
  ],
  "total_count": 2,
  "query_time_ms": 15
}
```

## 4. Prompts（提示）接口

### 4.1 分析文件关系提示

**提示名称**：`analyze_file_relationships`

**提示定义**：
```json
{
  "name": "analyze_file_relationships",
  "description": "Analyze relationships and dependencies for a file",
  "arguments": [
    {
      "name": "file_path",
      "description": "Path to the file to analyze",
      "required": true
    }
  ]
}
```

**提示模板**：
```
分析文件 {{file_path}} 的关系和依赖：

基于以下元数据：
- 标签：{{tags}}
- 注释：{{comment}}
- 出向关系：{{outgoing_relationships}}
- 入向关系：{{incoming_relationships}}

请分析：
1. 该文件在项目中的作用和重要性
2. 与其他文件的依赖关系
3. 修改该文件可能的影响范围
4. 相关文件的建议查看顺序
```

### 4.2 重构建议提示

**提示名称**：`suggest_refactoring`

**提示定义**：
```json
{
  "name": "suggest_refactoring",
  "description": "Suggest refactoring opportunities based on file relationships",
  "arguments": [
    {
      "name": "scope",
      "description": "Refactoring scope (file, module, or project)",
      "required": true
    }
  ]
}
```

**提示模板**：
```
基于代码库关系分析，为 {{scope}} 范围提供重构建议：

当前状态分析：
- 文件数量：{{file_count}}
- 关系复杂度：{{relationship_complexity}}
- 标签分布：{{tag_distribution}}

重构建议：
1. 识别高耦合模块
2. 建议模块拆分方案
3. 优化依赖关系
4. 改进代码组织结构
```

## 5. 错误处理

### 5.1 标准错误格式

```json
{
  "error": {
    "code": -32600,
    "message": "Invalid Request",
    "data": {
      "type": "ValidationError",
      "details": "File path 'invalid/path' does not exist",
      "timestamp": "2025-07-01T15:30:00Z"
    }
  }
}
```

### 5.2 错误代码定义

- `-32600`：Invalid Request - 请求格式错误
- `-32601`：Method not found - 方法不存在
- `-32602`：Invalid params - 参数无效
- `-32603`：Internal error - 内部错误
- `-32000`：File not found - 文件不存在
- `-32001`：Permission denied - 权限不足
- `-32002`：Validation failed - 数据验证失败

---

**相关文档**：
- [项目概述](./项目概述.md) - 了解功能背景
- [技术架构](./技术架构.md) - 查看实现架构
- [数据存储设计](./数据存储设计.md) - 了解数据结构

**文档版本**：v2.0  
**创建日期**：2025-07-01  
**最后更新**：2025-07-01
