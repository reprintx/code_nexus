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
use std::fs::OpenOptions;
use std::io::Write;
use chrono::Local;

/// 调试开关常量
const DEBUG_ENABLED: bool = false;

/// 写入调试日志到文件
fn write_debug_log(message: &str, project_path: Option<&str>) {
    if !DEBUG_ENABLED {
        return;
    }

    // 获取当前时间戳
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");

    // 构造日志条目
    let log_entry = format!("[{}] DEBUG: {}\n", timestamp, message);

    // 确定日志文件路径
    let log_file_path = if let Some(proj_path) = project_path {
        // 如果有项目路径，尝试写入到项目的数据目录
        if let Ok(validated_path) = validate_project_path(proj_path) {
            let data_dir = get_data_dir(&validated_path);
            // 确保数据目录存在
            if std::fs::create_dir_all(&data_dir).is_ok() {
                data_dir.join("debug.log")
            } else {
                std::path::PathBuf::from("debug.log")
            }
        } else {
            std::path::PathBuf::from("debug.log")
        }
    } else {
        // 没有项目路径时，写入到当前目录
        std::path::PathBuf::from("debug.log")
    };

    // 尝试写入日志文件
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
    {
        let _ = file.write_all(log_entry.as_bytes());
    } else {
        // 如果文件写入失败，回退到stderr
        eprintln!("{}", log_entry.trim_end());
    }
}



/// 调试信息输出宏（带项目路径）
macro_rules! debug_log_with_project {
    ($project_path:expr, $($arg:tt)*) => {
        if DEBUG_ENABLED {
            let message = format!($($arg)*);
            write_debug_log(&message, Some($project_path));
        }
    };
}

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
        debug_log_with_project!(project_path, "开始创建项目管理器，原始路径: {}", project_path);

        let validated_path = validate_project_path(project_path)?;
        debug_log_with_project!(project_path, "项目路径验证成功: {}", validated_path.display());

        let data_dir = get_data_dir(&validated_path);
        debug_log_with_project!(project_path, "数据目录路径: {}", data_dir.display());

        let storage = JsonStorage::new(&data_dir);
        storage.initialize().await?;
        debug_log_with_project!(project_path, "存储初始化完成");

        // 创建管理器
        debug_log_with_project!(project_path, "开始创建各种管理器");
        let mut tag_manager = TagManager::new(storage.clone());
        let mut comment_manager = CommentManager::new(storage.clone());
        let mut relation_manager = RelationManager::new(storage);

        // 初始化管理器
        debug_log_with_project!(project_path, "开始初始化管理器");
        tag_manager.initialize().await?;
        debug_log_with_project!(project_path, "标签管理器初始化完成");
        comment_manager.initialize().await?;
        debug_log_with_project!(project_path, "注释管理器初始化完成");
        relation_manager.initialize().await?;
        debug_log_with_project!(project_path, "关联关系管理器初始化完成");

        // 包装为 Arc<Mutex<>>
        debug_log_with_project!(project_path, "包装管理器为 Arc<Mutex<>>");
        let tag_manager = Arc::new(Mutex::new(tag_manager));
        let comment_manager = Arc::new(Mutex::new(comment_manager));
        let relation_manager = Arc::new(Mutex::new(relation_manager));

        // 创建查询引擎
        debug_log_with_project!(project_path, "创建查询引擎");
        let query_engine = Arc::new(QueryEngine::new(
            tag_manager.clone(),
            comment_manager.clone(),
            relation_manager.clone(),
        ));

        debug_log_with_project!(project_path, "项目管理器创建完成: {}", project_path);
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
        debug_log_with_project!(project_path, "获取或创建项目管理器: {}", project_path);
        let mut projects = self.projects.lock().await;

        if let Some(project) = projects.get(project_path) {
            debug_log_with_project!(project_path, "找到已存在的项目管理器: {}", project_path);
            return Ok(project.clone());
        }

        // 创建新的项目管理器
        debug_log_with_project!(project_path, "项目管理器不存在，开始创建新的: {}", project_path);
        let project_manager = ProjectManager::new(project_path).await
            .map_err(|e| ErrorData::internal_error(format!("创建项目管理器失败: {}", e), None))?;

        let project_arc = Arc::new(Mutex::new(project_manager));
        projects.insert(project_path.to_string(), project_arc.clone());

        info!("为项目创建了新的管理器: {}", project_path);
        debug_log_with_project!(project_path, "项目管理器创建并缓存完成: {}", project_path);
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
        debug_log_with_project!(&params.project_path, "添加文件标签 - 项目路径: {}, 文件路径: {}, 标签: {:?}",
                   params.project_path, params.file_path, params.tags);

        // 验证文件路径
        let validated_path = match validate_project_path(&params.project_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "项目路径验证成功: {}", path.display());
                path
            },
            Err(e) => {
                debug_log_with_project!(&params.project_path, "项目路径验证失败: {}", e);
                return format!("项目路径验证失败: {}", e);
            },
        };

        let full_file_path = match validate_file_path(&validated_path, &params.file_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "文件路径验证成功: {}", path.display());
                path
            },
            Err(e) => {
                debug_log_with_project!(&params.project_path, "文件路径验证失败: {}", e);
                return format!("文件路径验证失败: {}", e);
            },
        };

        // 规范化文件路径
        let normalized_path = match normalize_file_path(&validated_path, &full_file_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "路径规范化成功: {}", path);
                path
            },
            Err(e) => {
                debug_log_with_project!(&params.project_path, "路径规范化失败: {}", e);
                return format!("路径规范化失败: {}", e);
            },
        };

        // 获取项目管理器并执行操作
        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => {
                debug_log_with_project!(&params.project_path, "获取项目管理器成功");
                pm
            },
            Err(e) => {
                debug_log_with_project!(&params.project_path, "获取项目管理器失败: {:?}", e);
                return format!("错误: {:?}", e);
            },
        };

        let pm = project_manager.lock().await;
        let result = pm.tag_manager.lock().await.add_tags(&full_file_path, &normalized_path, params.tags).await;

        match result {
            Ok(_) => {
                debug_log_with_project!(&params.project_path, "标签添加成功");
                self.format_success_response("标签添加成功")
            },
            Err(e) => {
                debug_log_with_project!(&params.project_path, "添加标签失败: {}", e);
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
        debug_log_with_project!(&params.project_path, "移除文件标签 - 项目路径: {}, 文件路径: {}, 标签: {:?}",
                   params.project_path, params.file_path, params.tags);

        // 验证项目路径
        let validated_path = match validate_project_path(&params.project_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "项目路径验证成功: {}", path.display());
                path
            },
            Err(e) => return format!("项目路径验证失败: {}", e),
        };

        // 对于删除操作，不验证文件是否存在，因为文件可能已被删除但数据库中还有记录
        let full_file_path = validated_path.join(&params.file_path);
        debug_log_with_project!(&params.project_path, "构建文件路径: {}", full_file_path.display());

        let normalized_path = match normalize_file_path(&validated_path, &full_file_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "路径规范化成功: {}", path);
                path
            },
            Err(e) => return format!("路径规范化失败: {}", e),
        };

        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let result = pm.tag_manager.lock().await.remove_tags(&full_file_path, &normalized_path, params.tags).await;

        match result {
            Ok(_) => {
                debug_log_with_project!(&params.project_path, "标签移除成功");
                self.format_success_response("标签移除成功")
            },
            Err(e) => {
                debug_log_with_project!(&params.project_path, "移除标签失败: {}", e);
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
        debug_log_with_project!(&params.project_path, "标签查询 - 项目路径: {}, 查询表达式: {}", params.project_path, params.query);

        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => {
                debug_log_with_project!(&params.project_path, "获取项目管理器成功");
                pm
            },
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        debug_log_with_project!(&params.project_path, "开始执行标签查询");
        let result = pm.query_engine.execute_tag_query(&params.query).await;

        match result {
            Ok(result) => {
                debug_log_with_project!(&params.project_path, "标签查询成功，返回{}个结果", result.files.len());
                self.format_data_response(&result)
            },
            Err(e) => {
                debug_log_with_project!(&params.project_path, "标签查询失败: {}", e);
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
        debug_log_with_project!(&params.project_path, "获取所有标签 - 项目路径: {}", params.project_path);

        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => {
                debug_log_with_project!(&params.project_path, "获取项目管理器成功");
                pm
            },
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        debug_log_with_project!(&params.project_path, "开始获取所有标签");
        let all_tags = pm.tag_manager.lock().await.get_all_tags();
        debug_log_with_project!(&params.project_path, "获取到标签数量: {}", all_tags.len());
        self.format_data_response(&all_tags)
    }

    /// 为文件添加注释
    #[tool(description = "为文件添加注释")]
    async fn add_file_comment(
        &self,
        #[tool(aggr)] params: AddCommentParams,
    ) -> String {
        debug_log_with_project!(&params.project_path, "添加文件注释 - 项目路径: {}, 文件路径: {}, 注释长度: {}",
                   params.project_path, params.file_path, params.comment.len());

        // 验证路径
        let validated_path = match validate_project_path(&params.project_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "项目路径验证成功: {}", path.display());
                path
            },
            Err(e) => return format!("项目路径验证失败: {}", e),
        };

        let full_file_path = match validate_file_path(&validated_path, &params.file_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "文件路径验证成功: {}", path.display());
                path
            },
            Err(e) => return format!("文件路径验证失败: {}", e),
        };

        let normalized_path = match normalize_file_path(&validated_path, &full_file_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "路径规范化成功: {}", path);
                path
            },
            Err(e) => return format!("路径规范化失败: {}", e),
        };

        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let result = pm.comment_manager.lock().await.add_comment(&full_file_path, &normalized_path, &params.comment).await;

        match result {
            Ok(_) => {
                debug_log_with_project!(&params.project_path, "注释添加成功");
                self.format_success_response("注释添加成功")
            },
            Err(e) => {
                debug_log_with_project!(&params.project_path, "添加注释失败: {}", e);
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
        debug_log_with_project!(&params.project_path, "更新文件注释 - 项目路径: {}, 文件路径: {}, 注释长度: {}",
                   params.project_path, params.file_path, params.comment.len());

        // 验证路径
        let validated_path = match validate_project_path(&params.project_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "项目路径验证成功: {}", path.display());
                path
            },
            Err(e) => return format!("项目路径验证失败: {}", e),
        };

        let full_file_path = match validate_file_path(&validated_path, &params.file_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "文件路径验证成功: {}", path.display());
                path
            },
            Err(e) => return format!("文件路径验证失败: {}", e),
        };

        let normalized_path = match normalize_file_path(&validated_path, &full_file_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "路径规范化成功: {}", path);
                path
            },
            Err(e) => return format!("路径规范化失败: {}", e),
        };

        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let result = pm.comment_manager.lock().await.update_comment(&full_file_path, &normalized_path, &params.comment).await;

        match result {
            Ok(_) => {
                debug_log_with_project!(&params.project_path, "注释更新成功");
                self.format_success_response("注释更新成功")
            },
            Err(e) => {
                debug_log_with_project!(&params.project_path, "更新注释失败: {}", e);
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
        debug_log_with_project!(&params.project_path, "添加文件关联关系 - 项目路径: {}, 源文件: {}, 目标文件: {}, 描述: {}",
                   params.project_path, params.from_file, params.to_file, params.description);

        // 验证路径
        let validated_path = match validate_project_path(&params.project_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "项目路径验证成功: {}", path.display());
                path
            },
            Err(e) => return format!("项目路径验证失败: {}", e),
        };

        let from_file_path = match validate_file_path(&validated_path, &params.from_file) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "源文件路径验证成功: {}", path.display());
                path
            },
            Err(e) => return format!("源文件路径验证失败: {}", e),
        };

        let to_file_path = match validate_file_path(&validated_path, &params.to_file) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "目标文件路径验证成功: {}", path.display());
                path
            },
            Err(e) => return format!("目标文件路径验证失败: {}", e),
        };

        let normalized_from = match normalize_file_path(&validated_path, &from_file_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "源文件路径规范化成功: {}", path);
                path
            },
            Err(e) => return format!("源文件路径规范化失败: {}", e),
        };

        let normalized_to = match normalize_file_path(&validated_path, &to_file_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "目标文件路径规范化成功: {}", path);
                path
            },
            Err(e) => return format!("目标文件路径规范化失败: {}", e),
        };

        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let result = pm.relation_manager.lock().await.add_relation(
            &from_file_path, &normalized_from,
            &to_file_path, &normalized_to,
            &params.description
        ).await;

        match result {
            Ok(_) => {
                debug_log_with_project!(&params.project_path, "关联关系添加成功");
                self.format_success_response("关联关系添加成功")
            },
            Err(e) => {
                debug_log_with_project!(&params.project_path, "添加关联关系失败: {}", e);
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
        debug_log_with_project!(&params.project_path, "移除文件关联关系 - 项目路径: {}, 源文件: {}, 目标文件: {}",
                   params.project_path, params.from_file, params.to_file);

        // 验证项目路径
        let validated_path = match validate_project_path(&params.project_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "项目路径验证成功: {}", path.display());
                path
            },
            Err(e) => return format!("项目路径验证失败: {}", e),
        };

        // 对于删除操作，不验证文件是否存在，因为文件可能已被删除但数据库中还有记录
        let from_file_path = validated_path.join(&params.from_file);
        let to_file_path = validated_path.join(&params.to_file);
        debug_log_with_project!(&params.project_path, "构建源文件路径: {}", from_file_path.display());
        debug_log_with_project!(&params.project_path, "构建目标文件路径: {}", to_file_path.display());

        let normalized_from = match normalize_file_path(&validated_path, &from_file_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "源文件路径规范化成功: {}", path);
                path
            },
            Err(e) => return format!("源文件路径规范化失败: {}", e),
        };

        let normalized_to = match normalize_file_path(&validated_path, &to_file_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "目标文件路径规范化成功: {}", path);
                path
            },
            Err(e) => return format!("目标文件路径规范化失败: {}", e),
        };

        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let result = pm.relation_manager.lock().await.remove_relation(
            &from_file_path, &normalized_from,
            &to_file_path, &normalized_to
        ).await;

        match result {
            Ok(_) => {
                debug_log_with_project!(&params.project_path, "关联关系移除成功");
                self.format_success_response("关联关系移除成功")
            },
            Err(e) => {
                debug_log_with_project!(&params.project_path, "移除关联关系失败: {}", e);
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
        debug_log_with_project!(&params.project_path, "查询文件关联关系 - 项目路径: {}, 文件路径: {}",
                   params.project_path, params.file_path);

        // 验证路径
        let validated_path = match validate_project_path(&params.project_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "项目路径验证成功: {}", path.display());
                path
            },
            Err(e) => return format!("项目路径验证失败: {}", e),
        };

        let full_file_path = match validate_file_path(&validated_path, &params.file_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "文件路径验证成功: {}", path.display());
                path
            },
            Err(e) => return format!("文件路径验证失败: {}", e),
        };

        let normalized_path = match normalize_file_path(&validated_path, &full_file_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "路径规范化成功: {}", path);
                path
            },
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
        debug_log_with_project!(&params.project_path, "查询入向关联关系 - 项目路径: {}, 文件路径: {}",
                   params.project_path, params.file_path);

        // 验证路径
        let validated_path = match validate_project_path(&params.project_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "项目路径验证成功: {}", path.display());
                path
            },
            Err(e) => return format!("项目路径验证失败: {}", e),
        };

        let full_file_path = match validate_file_path(&validated_path, &params.file_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "文件路径验证成功: {}", path.display());
                path
            },
            Err(e) => return format!("文件路径验证失败: {}", e),
        };

        let normalized_path = match normalize_file_path(&validated_path, &full_file_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "路径规范化成功: {}", path);
                path
            },
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
        debug_log_with_project!(&params.project_path, "获取文件信息 - 项目路径: {}, 文件路径: {}",
                   params.project_path, params.file_path);

        // 验证路径
        let validated_path = match validate_project_path(&params.project_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "项目路径验证成功: {}", path.display());
                path
            },
            Err(e) => return format!("项目路径验证失败: {}", e),
        };

        let full_file_path = match validate_file_path(&validated_path, &params.file_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "文件路径验证成功: {}", path.display());
                path
            },
            Err(e) => return format!("文件路径验证失败: {}", e),
        };

        let normalized_path = match normalize_file_path(&validated_path, &full_file_path) {
            Ok(path) => {
                debug_log_with_project!(&params.project_path, "路径规范化成功: {}", path);
                path
            },
            Err(e) => return format!("路径规范化失败: {}", e),
        };

        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => pm,
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        let result = pm.query_engine.get_file_info(&normalized_path).await;

        match result {
            Ok(file_info) => {
                debug_log_with_project!(&params.project_path, "获取文件信息成功");
                self.format_data_response(&file_info)
            },
            Err(e) => {
                debug_log_with_project!(&params.project_path, "获取文件信息失败: {}", e);
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
        debug_log_with_project!(&params.project_path, "获取系统状态 - 项目路径: {}", params.project_path);

        let project_manager = match self.get_or_create_project(&params.project_path).await {
            Ok(pm) => {
                debug_log_with_project!(&params.project_path, "获取项目管理器成功");
                pm
            },
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        debug_log_with_project!(&params.project_path, "开始获取系统状态");
        let result = pm.query_engine.get_system_status().await;

        match result {
            Ok(status) => {
                debug_log_with_project!(&params.project_path, "获取系统状态成功");
                self.format_data_response(&status)
            },
            Err(e) => {
                debug_log_with_project!(&params.project_path, "获取系统状态失败: {}", e);
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
        debug_log_with_project!(&project_path, "搜索文件 - 项目路径: {}, 关键词: {}", project_path, keyword);

        let project_manager = match self.get_or_create_project(&project_path).await {
            Ok(pm) => {
                debug_log_with_project!(&project_path, "获取项目管理器成功");
                pm
            },
            Err(e) => return format!("错误: {:?}", e),
        };

        let pm = project_manager.lock().await;
        debug_log_with_project!(&project_path, "开始执行搜索查询");
        let result = pm.query_engine.search_files(&keyword).await;

        match result {
            Ok(results) => {
                debug_log_with_project!(&project_path, "搜索文件成功，返回{}个结果", results.len());
                self.format_data_response(&results)
            },
            Err(e) => {
                debug_log_with_project!(&project_path, "搜索文件失败: {}", e);
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
