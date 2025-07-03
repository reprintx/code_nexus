use thiserror::Error;

/// CodeNexus 错误类型
#[derive(Error, Debug)]
pub enum CodeNexusError {
    #[error("文件不存在: {0}")]
    FileNotFound(String),

    #[error("标签格式错误: {0}，应为 type:value 格式")]
    InvalidTagFormat(String),

    #[error("查询语法错误: {0}")]
    InvalidQuerySyntax(String),

    #[error("关联关系已存在: {from} -> {to}")]
    RelationAlreadyExists { from: String, to: String },

    #[error("关联关系不存在: {from} -> {to}")]
    RelationNotFound { from: String, to: String },

    #[error("标签不存在: {tag} 在文件 {file}")]
    TagNotFound { tag: String, file: String },

    #[error("存储错误: {0}")]
    StorageError(#[from] std::io::Error),

    #[error("JSON 序列化错误: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("文件系统错误: {0}")]
    FileSystemError(String),

    #[error("配置错误: {0}")]
    ConfigError(String),

    #[error("内部错误: {0}")]
    InternalError(String),
}

/// 结果类型别名
pub type Result<T> = std::result::Result<T, CodeNexusError>;

impl CodeNexusError {
    /// 获取错误的恢复建议
    pub fn recovery_suggestion(&self) -> &'static str {
        match self {
            CodeNexusError::FileNotFound(_) => "请检查文件路径是否正确",
            CodeNexusError::InvalidTagFormat(_) => "请使用 type:value 格式，如 category:api",
            CodeNexusError::InvalidQuerySyntax(_) => "请检查查询语法，支持 AND、NOT、通配符",
            CodeNexusError::RelationAlreadyExists { .. } => "关联关系已存在，请先移除再添加",
            CodeNexusError::RelationNotFound { .. } => "请先添加关联关系",
            CodeNexusError::TagNotFound { .. } => "请先为文件添加该标签",
            CodeNexusError::StorageError(_) => "请检查文件权限和磁盘空间",
            CodeNexusError::SerializationError(_) => "数据格式错误，请检查数据文件",
            CodeNexusError::FileSystemError(_) => "请检查文件系统权限",
            CodeNexusError::ConfigError(_) => "请检查配置文件格式",
            CodeNexusError::InternalError(_) => "请重试或联系技术支持",
        }
    }

    /// 获取错误代码
    pub fn error_code(&self) -> &'static str {
        match self {
            CodeNexusError::FileNotFound(_) => "FILE_NOT_FOUND",
            CodeNexusError::InvalidTagFormat(_) => "INVALID_TAG_FORMAT",
            CodeNexusError::InvalidQuerySyntax(_) => "INVALID_QUERY_SYNTAX",
            CodeNexusError::RelationAlreadyExists { .. } => "RELATION_ALREADY_EXISTS",
            CodeNexusError::RelationNotFound { .. } => "RELATION_NOT_FOUND",
            CodeNexusError::TagNotFound { .. } => "TAG_NOT_FOUND",
            CodeNexusError::StorageError(_) => "STORAGE_ERROR",
            CodeNexusError::SerializationError(_) => "SERIALIZATION_ERROR",
            CodeNexusError::FileSystemError(_) => "FILESYSTEM_ERROR",
            CodeNexusError::ConfigError(_) => "CONFIG_ERROR",
            CodeNexusError::InternalError(_) => "INTERNAL_ERROR",
        }
    }
}

/// 格式化错误响应
pub fn format_error_response(error: &CodeNexusError) -> String {
    serde_json::json!({
        "error": {
            "code": error.error_code(),
            "message": error.to_string(),
            "suggestion": error.recovery_suggestion()
        }
    }).to_string()
}

/// 转换为 MCP ErrorData
impl From<CodeNexusError> for rmcp::model::ErrorData {
    fn from(error: CodeNexusError) -> Self {
        rmcp::model::ErrorData::internal_error(
            error.to_string(),
            Some(serde_json::json!({
                "code": error.error_code(),
                "suggestion": error.recovery_suggestion()
            }))
        )
    }
}
