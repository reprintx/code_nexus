use crate::error::{format_error_response, CodeNexusError};
use crate::managers::{TagManager, CommentManager, RelationManager};
use crate::models::*;
use crate::query::QueryEngine;
use crate::storage::JsonStorage;
use crate::utils::{validate_project_path, validate_file_path, get_data_dir, normalize_file_path};
use rmcp::{ServerHandler, model::{ServerInfo, ServerCapabilities, ErrorData}, tool};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

/// 项目管理器
#[derive(Debug)]
pub struct ProjectManager {
    tag_manager: Arc<Mutex<TagManager>>,
    comment_manager: Arc<Mutex<CommentManager>>,
    relation_manager: Arc<Mutex<RelationManager>>,
    query_engine: Arc<QueryEngine>,
    project_path: String,
}

/// CodeNexus MCP 服务器
#[derive(Debug, Clone)]
pub struct CodeNexusServer {
    // 使用 HashMap 管理多个项目
    projects: Arc<Mutex<HashMap<String, Arc<Mutex<ProjectManager>>>>>,
}

impl ProjectManager {
    /// 创建新的项目管理器
    pub async fn new(project_path: &str) -> std::result::Result<Self, CodeNexusError> {
        let validated_path = validate_project_path(project_path)?;
        let data_dir = get_data_dir(&validated_path);

        let storage = JsonStorage::new(&data_dir);
        storage.initialize().await?;

        // 创建管理器
        let mut tag_manager = TagManager::new(storage.clone());
        let mut comment_manager = CommentManager::new(storage.clone());
        let mut relation_manager = RelationManager::new(storage);

        // 初始化管理器
        tag_manager.initialize().await?;
        comment_manager.initialize().await?;
        relation_manager.initialize().await?;

        // 包装为 Arc<Mutex<>>
        let tag_manager = Arc::new(Mutex::new(tag_manager));
        let comment_manager = Arc::new(Mutex::new(comment_manager));
        let relation_manager = Arc::new(Mutex::new(relation_manager));

        // 创建查询引擎
        let query_engine = Arc::new(QueryEngine::new(
            tag_manager.clone(),
            comment_manager.clone(),
            relation_manager.clone(),
        ));

        Ok(Self {
            tag_manager,
            comment_manager,
            relation_manager,
            query_engine,
            project_path: project_path.to_string(),
        })
    }
}

impl CodeNexusServer {
    /// 创建新的服务器实例
    pub async fn new() -> std::result::Result<Self, ErrorData> {
        info!("CodeNexus 服务器初始化完成");

        Ok(Self {
            projects: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// 获取或创建项目管理器
    pub async fn get_or_create_project(&self, project_path: &str) -> std::result::Result<Arc<Mutex<ProjectManager>>, ErrorData> {
        let mut projects = self.projects.lock().await;

        if let Some(project) = projects.get(project_path) {
            return Ok(project.clone());
        }

        // 创建新的项目管理器
        let project_manager = ProjectManager::new(project_path).await
            .map_err(|e| ErrorData::internal_error(format!("创建项目管理器失败: {}", e), None))?;

        let project_arc = Arc::new(Mutex::new(project_manager));
        projects.insert(project_path.to_string(), project_arc.clone());

        info!("为项目创建了新的管理器: {}", project_path);
        Ok(project_arc)
    }

    /// 执行项目操作的辅助方法
    async fn execute_project_operation<F, R>(&self, project_path: &str, operation: F) -> String
    where
        F: FnOnce(Arc<Mutex<ProjectManager>>) -> R + Send,
        R: std::future::Future<Output = String> + Send,
    {
        let project_manager = match self.get_or_create_project(project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        operation(project_manager).await
    }

    /// 格式化成功响应
    fn format_success_response(&self, message: &str) -> String {
        serde_json::json!({
            "success": true,
            "message": message
        }).to_string()
    }

    /// 格式化数据响应
    fn format_data_response<T: serde::Serialize>(&self, data: &T) -> String {
        match serde_json::to_string(data) {
            Ok(json) => json,
            Err(e) => {
                error!("序列化响应数据失败: {}", e);
                format_error_response(&CodeNexusError::SerializationError(e))
            }
        }
    }
}

#[tool(tool_box)]
impl CodeNexusServer {
    /// 为文件添加标签
    #[tool(description = "为文件添加标签，标签格式为 type:value")]
    async fn add_file_tags(
        &self,
        #[tool(aggr)] params: AddTagsParams,
    ) -> String {
        // 验证文件路径
        let validated_path = match validate_project_path(&params.project_path) {
            Ok(path) => path,
            Err(e) => return format!("项目路径验证失败: {}", e),
        };

        let full_file_path = match validate_file_path(&validated_path, &params.file_path) {
            Ok(path) => path,
            Err(e) => return format!("文件路径验证失败: {}", e),
        };

        // 规范化文件路径
        let normalized_path = match normalize_file_path(&validated_path, &full_file_path) {
            Ok(path) => path,
            Err(e) => return format!("路径规范化失败: {}", e),
        };

        // 获取项目管理器并执行操作
        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let result = pm.tag_manager.lock().await.add_tags(&normalized_path, params.tags).await;

        match result {
            Ok(_) => self.format_success_response("标签添加成功"),
            Err(e) => {
                error!("添加标签失败: {}", e);
                format_error_response(&e)
            }
        }
    }

    /// 移除文件标签
    #[tool(description = "移除文件的指定标签")]
    async fn remove_file_tags(
        &self,
        #[tool(aggr)] params: RemoveTagsParams,
    ) -> String {
        // 验证路径并获取项目管理器
        let validated_path = match validate_project_path(&params.project_path) {
            Ok(path) => path,
            Err(e) => return format!("项目路径验证失败: {}", e),
        };

        let full_file_path = match validate_file_path(&validated_path, &params.file_path) {
            Ok(path) => path,
            Err(e) => return format!("文件路径验证失败: {}", e),
        };

        let normalized_path = match normalize_file_path(&validated_path, &full_file_path) {
            Ok(path) => path,
            Err(e) => return format!("路径规范化失败: {}", e),
        };

        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let result = pm.tag_manager.lock().await.remove_tags(&normalized_path, params.tags).await;

        match result {
            Ok(_) => self.format_success_response("标签移除成功"),
            Err(e) => {
                error!("移除标签失败: {}", e);
                format_error_response(&e)
            }
        }
    }

    /// 根据标签查询文件
    #[tool(description = "根据标签查询文件，支持 AND、NOT、通配符")]
    async fn query_files_by_tags(
        &self,
        #[tool(aggr)] params: TagQueryParams,
    ) -> String {
        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let result = pm.query_engine.execute_tag_query(&params.query).await;

        match result {
            Ok(result) => self.format_data_response(&result),
            Err(e) => {
                error!("标签查询失败: {}", e);
                format_error_response(&e)
            }
        }
    }

    /// 获取所有标签
    #[tool(description = "获取所有标签，按类型分组")]
    async fn get_all_tags(
        &self,
        #[tool(aggr)] params: ProjectPathParams,
    ) -> String {
        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let all_tags = pm.tag_manager.lock().await.get_all_tags();
        self.format_data_response(&all_tags)
    }

    /// 为文件添加注释
    #[tool(description = "为文件添加注释")]
    async fn add_file_comment(
        &self,
        #[tool(aggr)] params: AddCommentParams,
    ) -> String {
        // 验证路径
        let validated_path = match validate_project_path(&params.project_path) {
            Ok(path) => path,
            Err(e) => return format!("项目路径验证失败: {}", e),
        };

        let full_file_path = match validate_file_path(&validated_path, &params.file_path) {
            Ok(path) => path,
            Err(e) => return format!("文件路径验证失败: {}", e),
        };

        let normalized_path = match normalize_file_path(&validated_path, &full_file_path) {
            Ok(path) => path,
            Err(e) => return format!("路径规范化失败: {}", e),
        };

        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let result = pm.comment_manager.lock().await.add_comment(&normalized_path, &params.comment).await;

        match result {
            Ok(_) => self.format_success_response("注释添加成功"),
            Err(e) => {
                error!("添加注释失败: {}", e);
                format_error_response(&e)
            }
        }
    }

    /// 更新文件注释
    #[tool(description = "更新文件注释")]
    async fn update_file_comment(
        &self,
        #[tool(aggr)] params: AddCommentParams,
    ) -> String {
        // 验证路径
        let validated_path = match validate_project_path(&params.project_path) {
            Ok(path) => path,
            Err(e) => return format!("项目路径验证失败: {}", e),
        };

        let full_file_path = match validate_file_path(&validated_path, &params.file_path) {
            Ok(path) => path,
            Err(e) => return format!("文件路径验证失败: {}", e),
        };

        let normalized_path = match normalize_file_path(&validated_path, &full_file_path) {
            Ok(path) => path,
            Err(e) => return format!("路径规范化失败: {}", e),
        };

        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let result = pm.comment_manager.lock().await.update_comment(&normalized_path, &params.comment).await;

        match result {
            Ok(_) => self.format_success_response("注释更新成功"),
            Err(e) => {
                error!("更新注释失败: {}", e);
                format_error_response(&e)
            }
        }
    }

    /// 添加文件关联关系
    #[tool(description = "添加文件间的关联关系")]
    async fn add_file_relation(
        &self,
        #[tool(aggr)] params: AddRelationParams,
    ) -> String {
        // 验证路径
        let validated_path = match validate_project_path(&params.project_path) {
            Ok(path) => path,
            Err(e) => return format!("项目路径验证失败: {}", e),
        };

        let from_file_path = match validate_file_path(&validated_path, &params.from_file) {
            Ok(path) => path,
            Err(e) => return format!("源文件路径验证失败: {}", e),
        };

        let to_file_path = match validate_file_path(&validated_path, &params.to_file) {
            Ok(path) => path,
            Err(e) => return format!("目标文件路径验证失败: {}", e),
        };

        let normalized_from = match normalize_file_path(&validated_path, &from_file_path) {
            Ok(path) => path,
            Err(e) => return format!("源文件路径规范化失败: {}", e),
        };

        let normalized_to = match normalize_file_path(&validated_path, &to_file_path) {
            Ok(path) => path,
            Err(e) => return format!("目标文件路径规范化失败: {}", e),
        };

        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let result = pm.relation_manager.lock().await.add_relation(
            &normalized_from,
            &normalized_to,
            &params.description
        ).await;

        match result {
            Ok(_) => self.format_success_response("关联关系添加成功"),
            Err(e) => {
                error!("添加关联关系失败: {}", e);
                format_error_response(&e)
            }
        }
    }

    /// 移除文件关联关系
    #[tool(description = "移除文件间的关联关系")]
    async fn remove_file_relation(
        &self,
        #[tool(aggr)] params: RemoveRelationParams,
    ) -> String {
        // 验证路径
        let validated_path = match validate_project_path(&params.project_path) {
            Ok(path) => path,
            Err(e) => return format!("项目路径验证失败: {}", e),
        };

        let from_file_path = match validate_file_path(&validated_path, &params.from_file) {
            Ok(path) => path,
            Err(e) => return format!("源文件路径验证失败: {}", e),
        };

        let to_file_path = match validate_file_path(&validated_path, &params.to_file) {
            Ok(path) => path,
            Err(e) => return format!("目标文件路径验证失败: {}", e),
        };

        let normalized_from = match normalize_file_path(&validated_path, &from_file_path) {
            Ok(path) => path,
            Err(e) => return format!("源文件路径规范化失败: {}", e),
        };

        let normalized_to = match normalize_file_path(&validated_path, &to_file_path) {
            Ok(path) => path,
            Err(e) => return format!("目标文件路径规范化失败: {}", e),
        };

        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let result = pm.relation_manager.lock().await.remove_relation(&normalized_from, &normalized_to).await;

        match result {
            Ok(_) => self.format_success_response("关联关系移除成功"),
            Err(e) => {
                error!("移除关联关系失败: {}", e);
                format_error_response(&e)
            }
        }
    }

    /// 查询文件关联关系
    #[tool(description = "查询文件的出向关联关系")]
    async fn query_file_relations(
        &self,
        #[tool(aggr)] params: FilePathParams,
    ) -> String {
        // 验证路径
        let validated_path = match validate_project_path(&params.project_path) {
            Ok(path) => path,
            Err(e) => return format!("项目路径验证失败: {}", e),
        };

        let full_file_path = match validate_file_path(&validated_path, &params.file_path) {
            Ok(path) => path,
            Err(e) => return format!("文件路径验证失败: {}", e),
        };

        let normalized_path = match normalize_file_path(&validated_path, &full_file_path) {
            Ok(path) => path,
            Err(e) => return format!("路径规范化失败: {}", e),
        };

        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let relations = pm.relation_manager.lock().await.get_file_relations(&normalized_path);
        self.format_data_response(&relations)
    }

    /// 查询入向关联关系
    #[tool(description = "查询指向该文件的关联关系")]
    async fn query_incoming_relations(
        &self,
        #[tool(aggr)] params: FilePathParams,
    ) -> String {
        // 验证路径
        let validated_path = match validate_project_path(&params.project_path) {
            Ok(path) => path,
            Err(e) => return format!("项目路径验证失败: {}", e),
        };

        let full_file_path = match validate_file_path(&validated_path, &params.file_path) {
            Ok(path) => path,
            Err(e) => return format!("文件路径验证失败: {}", e),
        };

        let normalized_path = match normalize_file_path(&validated_path, &full_file_path) {
            Ok(path) => path,
            Err(e) => return format!("路径规范化失败: {}", e),
        };

        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let relations = pm.relation_manager.lock().await.get_incoming_relations(&normalized_path);
        self.format_data_response(&relations)
    }

    /// 获取文件完整信息
    #[tool(description = "获取文件的完整信息，包括标签、注释、关联关系")]
    async fn get_file_info(
        &self,
        #[tool(aggr)] params: FilePathParams,
    ) -> String {
        // 验证路径
        let validated_path = match validate_project_path(&params.project_path) {
            Ok(path) => path,
            Err(e) => return format!("项目路径验证失败: {}", e),
        };

        let full_file_path = match validate_file_path(&validated_path, &params.file_path) {
            Ok(path) => path,
            Err(e) => return format!("文件路径验证失败: {}", e),
        };

        let normalized_path = match normalize_file_path(&validated_path, &full_file_path) {
            Ok(path) => path,
            Err(e) => return format!("路径规范化失败: {}", e),
        };

        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let result = pm.query_engine.get_file_info(&normalized_path).await;

        match result {
            Ok(file_info) => self.format_data_response(&file_info),
            Err(e) => {
                error!("获取文件信息失败: {}", e);
                format_error_response(&e)
            }
        }
    }

    /// 获取系统状态
    #[tool(description = "获取系统状态和统计信息")]
    async fn get_system_status(
        &self,
        #[tool(aggr)] params: ProjectPathParams,
    ) -> String {
        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let result = pm.query_engine.get_system_status().await;

        match result {
            Ok(status) => self.format_data_response(&status),
            Err(e) => {
                error!("获取系统状态失败: {}", e);
                format_error_response(&e)
            }
        }
    }

    /// 搜索文件
    #[tool(description = "综合搜索文件，包括注释和关联关系描述")]
    async fn search_files(
        &self,
        #[tool(param)]
        #[schemars(description = "项目根目录路径")]
        project_path: String,
        #[tool(param)]
        #[schemars(description = "搜索关键词")]
        keyword: String,
    ) -> String {
        let project_manager = match self.get_or_create_project(&project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let result = pm.query_engine.search_files(&keyword).await;

        match result {
            Ok(results) => self.format_data_response(&results),
            Err(e) => {
                error!("搜索文件失败: {}", e);
                format_error_response(&e)
            }
        }
    }
}

#[tool(tool_box)]
impl ServerHandler for CodeNexusServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("CodeNexus 代码库关系管理工具 - 通过标签、注释和关联关系管理代码文件".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
