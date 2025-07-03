use crate::error::{CodeNexusError, Result};
use crate::storage::{JsonStorage, TagsData};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::{debug, info};

/// 标签管理器
#[derive(Debug)]
pub struct TagManager {
    storage: JsonStorage,
    // 内存索引
    file_tags: HashMap<String, HashSet<String>>,
    tag_index: HashMap<String, HashSet<String>>, // tag_type -> tag_values
    tag_to_files: HashMap<String, HashSet<String>>, // tag -> files
}

impl TagManager {
    /// 创建新的标签管理器
    pub fn new(storage: JsonStorage) -> Self {
        Self {
            storage,
            file_tags: HashMap::new(),
            tag_index: HashMap::new(),
            tag_to_files: HashMap::new(),
        }
    }

    /// 初始化管理器，加载数据到内存
    pub async fn initialize(&mut self) -> Result<()> {
        let data = self.storage.load_tags().await?;
        self.build_indices(&data);
        info!("标签管理器初始化完成，加载了 {} 个文件的标签", self.file_tags.len());
        Ok(())
    }

    /// 构建内存索引
    fn build_indices(&mut self, data: &TagsData) {
        self.file_tags.clear();
        self.tag_index.clear();
        self.tag_to_files.clear();

        for (file_path, tags) in &data.file_tags {
            let tag_set: HashSet<String> = tags.iter().cloned().collect();
            self.file_tags.insert(file_path.clone(), tag_set);

            for tag in tags {
                self.update_indices(tag, file_path);
            }
        }
    }

    /// 更新内存索引
    fn update_indices(&mut self, tag: &str, file_path: &str) {
        // 更新标签类型索引
        if let Some((tag_type, tag_value)) = tag.split_once(':') {
            self.tag_index
                .entry(tag_type.to_string())
                .or_default()
                .insert(tag_value.to_string());
        }

        // 更新标签到文件映射
        self.tag_to_files
            .entry(tag.to_string())
            .or_default()
            .insert(file_path.to_string());
    }

    /// 移除索引中的标签
    fn remove_from_indices(&mut self, tag: &str, file_path: &str) {
        // 从标签到文件映射中移除
        if let Some(files) = self.tag_to_files.get_mut(tag) {
            files.remove(file_path);
            if files.is_empty() {
                self.tag_to_files.remove(tag);
                
                // 如果没有文件使用这个标签，从标签类型索引中移除
                if let Some((tag_type, tag_value)) = tag.split_once(':') {
                    if let Some(values) = self.tag_index.get_mut(tag_type) {
                        values.remove(tag_value);
                        if values.is_empty() {
                            self.tag_index.remove(tag_type);
                        }
                    }
                }
            }
        }
    }

    /// 验证标签格式
    pub fn validate_tag(&self, tag: &str) -> Result<()> {
        if !tag.contains(':') {
            return Err(CodeNexusError::InvalidTagFormat(tag.to_string()));
        }

        let parts: Vec<&str> = tag.split(':').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(CodeNexusError::InvalidTagFormat(tag.to_string()));
        }

        Ok(())
    }

    /// 验证文件路径
    fn validate_file_path(&self, file_path: &str) -> Result<()> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(CodeNexusError::FileNotFound(file_path.to_string()));
        }
        Ok(())
    }

    /// 为文件添加标签
    pub async fn add_tags(&mut self, file_path: &str, tags: Vec<String>) -> Result<()> {
        // 验证文件路径
        self.validate_file_path(file_path)?;

        // 验证标签格式
        for tag in &tags {
            self.validate_tag(tag)?;
        }

        // 更新内存数据
        let mut added_tags = Vec::new();

        // 先获取或创建文件标签集合
        let file_tags = self.file_tags.entry(file_path.to_string()).or_default();

        for tag in tags {
            if file_tags.insert(tag.clone()) {
                added_tags.push(tag);
            }
        }

        // 更新索引（在借用结束后）
        for tag in &added_tags {
            self.update_indices(tag, file_path);
        }

        if !added_tags.is_empty() {
            // 保存到存储
            self.save_to_storage().await?;
            info!("为文件 {} 添加了 {} 个标签: {:?}", file_path, added_tags.len(), added_tags);
        } else {
            debug!("文件 {} 的标签没有变化", file_path);
        }

        Ok(())
    }

    /// 移除文件标签
    pub async fn remove_tags(&mut self, file_path: &str, tags: Vec<String>) -> Result<()> {
        // 先检查文件是否存在标签
        if !self.file_tags.contains_key(file_path) {
            return Err(CodeNexusError::FileNotFound(file_path.to_string()));
        }

        let mut removed_tags = Vec::new();

        // 验证所有标签都存在
        for tag in &tags {
            if let Some(file_tags) = self.file_tags.get(file_path) {
                if !file_tags.contains(tag) {
                    return Err(CodeNexusError::TagNotFound {
                        tag: tag.clone(),
                        file: file_path.to_string(),
                    });
                }
            }
        }

        // 移除标签
        if let Some(file_tags) = self.file_tags.get_mut(file_path) {
            for tag in tags {
                if file_tags.remove(&tag) {
                    removed_tags.push(tag);
                }
            }
        }

        // 更新索引
        for tag in &removed_tags {
            self.remove_from_indices(tag, file_path);
        }

        // 如果文件没有标签了，移除文件记录
        if let Some(file_tags) = self.file_tags.get(file_path) {
            if file_tags.is_empty() {
                self.file_tags.remove(file_path);
            }
        }

        if !removed_tags.is_empty() {
            self.save_to_storage().await?;
            info!("从文件 {} 移除了 {} 个标签: {:?}", file_path, removed_tags.len(), removed_tags);
        }

        Ok(())
    }

    /// 获取文件标签
    pub fn get_file_tags(&self, file_path: &str) -> Vec<String> {
        self.file_tags
            .get(file_path)
            .map(|tags| tags.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// 获取所有标签，按类型分组
    pub fn get_all_tags(&self) -> HashMap<String, Vec<String>> {
        self.tag_index
            .iter()
            .map(|(tag_type, tag_values)| {
                let mut values: Vec<String> = tag_values.iter().cloned().collect();
                values.sort();
                (tag_type.clone(), values)
            })
            .collect()
    }

    /// 根据标签查询文件
    pub fn query_files_by_tags(&self, query: &str) -> Result<Vec<String>> {
        // 简单的查询实现，支持单个标签和 AND 操作
        let query = query.trim();
        
        if query.is_empty() {
            return Ok(Vec::new());
        }

        // 处理 AND 查询
        if query.contains(" AND ") {
            let tags: Vec<&str> = query.split(" AND ").map(|s| s.trim()).collect();
            return self.query_files_with_and(&tags);
        }

        // 单个标签查询
        Ok(self.tag_to_files
            .get(query)
            .map(|files| files.iter().cloned().collect())
            .unwrap_or_default())
    }

    /// AND 查询实现
    fn query_files_with_and(&self, tags: &[&str]) -> Result<Vec<String>> {
        if tags.is_empty() {
            return Ok(Vec::new());
        }

        // 获取第一个标签的文件集合
        let mut result_files = self.tag_to_files
            .get(tags[0])
            .cloned()
            .unwrap_or_default();

        // 与其他标签的文件集合求交集
        for &tag in &tags[1..] {
            if let Some(tag_files) = self.tag_to_files.get(tag) {
                result_files = result_files.intersection(tag_files).cloned().collect();
            } else {
                return Ok(Vec::new()); // 如果任何标签不存在，结果为空
            }
        }

        let mut result: Vec<String> = result_files.into_iter().collect();
        result.sort();
        Ok(result)
    }

    /// 获取未标记的文件
    pub fn get_untagged_files(&self) -> Vec<String> {
        // 这里需要扫描文件系统，暂时返回空列表
        // 实际实现需要遍历项目文件并检查是否有标签
        Vec::new()
    }

    /// 保存数据到存储
    async fn save_to_storage(&self) -> Result<()> {
        let data = TagsData {
            file_tags: self.file_tags
                .iter()
                .map(|(path, tags)| (path.clone(), tags.iter().cloned().collect()))
                .collect(),
        };

        self.storage.save_tags(&data).await
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> (usize, usize, usize) {
        let total_files = self.file_tags.len();
        let total_tags = self.tag_to_files.len();
        let total_tag_types = self.tag_index.len();
        (total_files, total_tags, total_tag_types)
    }
}
