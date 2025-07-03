use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::collections::HashMap;

/// 文件完整信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub tags: Vec<String>,
    pub comment: Option<String>,
    pub relations: Vec<Relation>,
    pub incoming_relations: Vec<Relation>,
}

/// 文件关联关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub target: String,
    pub description: String,
}

/// 标签查询参数
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TagQueryParams {
    #[schemars(description = "标签查询表达式，支持 AND、NOT、通配符")]
    pub query: String,
}

/// 添加标签参数
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddTagsParams {
    #[schemars(description = "文件路径")]
    pub file_path: String,
    #[schemars(description = "标签列表，格式为 type:value")]
    pub tags: Vec<String>,
}

/// 移除标签参数
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveTagsParams {
    #[schemars(description = "文件路径")]
    pub file_path: String,
    #[schemars(description = "要移除的标签列表")]
    pub tags: Vec<String>,
}

/// 添加注释参数
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddCommentParams {
    #[schemars(description = "文件路径")]
    pub file_path: String,
    #[schemars(description = "注释内容")]
    pub comment: String,
}

/// 添加关联关系参数
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddRelationParams {
    #[schemars(description = "源文件路径")]
    pub from_file: String,
    #[schemars(description = "目标文件路径")]
    pub to_file: String,
    #[schemars(description = "关联关系描述")]
    pub description: String,
}

/// 移除关联关系参数
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveRelationParams {
    #[schemars(description = "源文件路径")]
    pub from_file: String,
    #[schemars(description = "目标文件路径")]
    pub to_file: String,
}

/// 文件路径参数
#[derive(Debug, Deserialize, JsonSchema)]
pub struct FilePathParams {
    #[schemars(description = "文件路径")]
    pub file_path: String,
}

/// 查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub files: Vec<String>,
    pub total: usize,
}

/// 标签统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagStats {
    pub tag_types: HashMap<String, Vec<String>>,
    pub total_files: usize,
    pub total_tags: usize,
}

/// 系统状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub total_files: usize,
    pub tagged_files: usize,
    pub commented_files: usize,
    pub total_relations: usize,
    pub tag_stats: TagStats,
}
