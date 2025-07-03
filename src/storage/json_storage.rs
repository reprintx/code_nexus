use crate::error::{CodeNexusError, Result};
use crate::models::Relation;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, error, info};

/// JSON 存储管理器
#[derive(Debug, Clone)]
pub struct JsonStorage {
    data_dir: PathBuf,
}

/// 标签数据结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TagsData {
    pub file_tags: HashMap<String, Vec<String>>,
}

/// 注释数据结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommentsData {
    pub file_comments: HashMap<String, String>,
}

/// 关联关系数据结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RelationsData {
    pub file_relations: HashMap<String, Vec<Relation>>,
}

impl JsonStorage {
    /// 创建新的存储实例
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
        }
    }

    /// 初始化存储目录
    pub async fn initialize(&self) -> Result<()> {
        if !self.data_dir.exists() {
            fs::create_dir_all(&self.data_dir).await?;
            info!("创建数据目录: {:?}", self.data_dir);
        }

        // 确保数据文件存在
        self.ensure_file_exists("tags.json", &TagsData::default()).await?;
        self.ensure_file_exists("comments.json", &CommentsData::default()).await?;
        self.ensure_file_exists("relations.json", &RelationsData::default()).await?;

        Ok(())
    }

    /// 确保文件存在，如果不存在则创建默认内容
    async fn ensure_file_exists<T: Serialize>(&self, filename: &str, default_data: &T) -> Result<()> {
        let file_path = self.data_dir.join(filename);
        if !file_path.exists() {
            let json_data = serde_json::to_string_pretty(default_data)?;
            fs::write(&file_path, json_data).await?;
            debug!("创建默认数据文件: {:?}", file_path);
        }
        Ok(())
    }

    /// 加载标签数据
    pub async fn load_tags(&self) -> Result<TagsData> {
        let file_path = self.data_dir.join("tags.json");
        self.load_json_file(&file_path).await
    }

    /// 保存标签数据
    pub async fn save_tags(&self, data: &TagsData) -> Result<()> {
        let file_path = self.data_dir.join("tags.json");
        self.save_json_file(&file_path, data).await
    }

    /// 加载注释数据
    pub async fn load_comments(&self) -> Result<CommentsData> {
        let file_path = self.data_dir.join("comments.json");
        self.load_json_file(&file_path).await
    }

    /// 保存注释数据
    pub async fn save_comments(&self, data: &CommentsData) -> Result<()> {
        let file_path = self.data_dir.join("comments.json");
        self.save_json_file(&file_path, data).await
    }

    /// 加载关联关系数据
    pub async fn load_relations(&self) -> Result<RelationsData> {
        let file_path = self.data_dir.join("relations.json");
        self.load_json_file(&file_path).await
    }

    /// 保存关联关系数据
    pub async fn save_relations(&self, data: &RelationsData) -> Result<()> {
        let file_path = self.data_dir.join("relations.json");
        self.save_json_file(&file_path, data).await
    }

    /// 通用 JSON 文件加载
    async fn load_json_file<T: for<'de> Deserialize<'de> + Default>(&self, file_path: &Path) -> Result<T> {
        match fs::read_to_string(file_path).await {
            Ok(content) => {
                if content.trim().is_empty() {
                    Ok(T::default())
                } else {
                    serde_json::from_str(&content).map_err(|e| {
                        error!("JSON 解析错误 {:?}: {}", file_path, e);
                        CodeNexusError::SerializationError(e)
                    })
                }
            }
            Err(e) => {
                error!("文件读取错误 {:?}: {}", file_path, e);
                Err(CodeNexusError::StorageError(e))
            }
        }
    }

    /// 通用 JSON 文件保存
    async fn save_json_file<T: Serialize>(&self, file_path: &Path, data: &T) -> Result<()> {
        // 创建备份
        if file_path.exists() {
            let backup_path = file_path.with_extension("json.bak");
            if let Err(e) = fs::copy(file_path, &backup_path).await {
                error!("创建备份失败 {:?}: {}", backup_path, e);
            }
        }

        // 保存数据
        let json_data = serde_json::to_string_pretty(data)?;
        fs::write(file_path, json_data).await.map_err(|e| {
            error!("文件写入错误 {:?}: {}", file_path, e);
            CodeNexusError::StorageError(e)
        })?;

        debug!("数据已保存到: {:?}", file_path);
        Ok(())
    }

    /// 获取数据目录路径
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// 检查存储是否已初始化
    pub async fn is_initialized(&self) -> bool {
        self.data_dir.exists()
            && self.data_dir.join("tags.json").exists()
            && self.data_dir.join("comments.json").exists()
            && self.data_dir.join("relations.json").exists()
    }
}
