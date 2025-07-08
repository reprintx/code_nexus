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

    /// 验证文件路径（使用绝对路径）
    fn validate_file_path(&self, absolute_file_path: &Path) -> Result<()> {
        if !absolute_file_path.exists() {
            return Err(CodeNexusError::FileNotFound(absolute_file_path.to_string_lossy().to_string()));
        }
        Ok(())
    }

    /// 为文件添加标签
    pub async fn add_tags(&mut self, absolute_file_path: &Path, relative_file_path: &str, tags: Vec<String>) -> Result<()> {
        // 验证文件路径（使用绝对路径）
        self.validate_file_path(absolute_file_path)?;

        // 验证标签格式
        for tag in &tags {
            self.validate_tag(tag)?;
        }

        // 更新内存数据（使用相对路径存储）
        let mut added_tags = Vec::new();

        // 先获取或创建文件标签集合
        let file_tags = self.file_tags.entry(relative_file_path.to_string()).or_default();

        for tag in tags {
            if file_tags.insert(tag.clone()) {
                added_tags.push(tag);
            }
        }

        // 更新索引（在借用结束后）
        for tag in &added_tags {
            self.update_indices(tag, relative_file_path);
        }

        if !added_tags.is_empty() {
            // 保存到存储
            self.save_to_storage().await?;
            info!("为文件 {} 添加了 {} 个标签: {:?}", relative_file_path, added_tags.len(), added_tags);
        } else {
            debug!("文件 {} 的标签没有变化", relative_file_path);
        }

        Ok(())
    }

    /// 移除文件标签
    pub async fn remove_tags(&mut self, _absolute_file_path: &Path, relative_file_path: &str, tags: Vec<String>) -> Result<()> {
        // 对于删除操作，不验证文件是否存在，因为文件可能已被删除但数据库中还有记录

        // 先检查文件是否存在标签（使用相对路径）
        if !self.file_tags.contains_key(relative_file_path) {
            return Err(CodeNexusError::FileNotFound(relative_file_path.to_string()));
        }

        let mut removed_tags = Vec::new();

        // 验证所有标签都存在
        for tag in &tags {
            if let Some(file_tags) = self.file_tags.get(relative_file_path) {
                if !file_tags.contains(tag) {
                    return Err(CodeNexusError::TagNotFound {
                        tag: tag.clone(),
                        file: relative_file_path.to_string(),
                    });
                }
            }
        }

        // 移除标签
        if let Some(file_tags) = self.file_tags.get_mut(relative_file_path) {
            for tag in tags {
                if file_tags.remove(&tag) {
                    removed_tags.push(tag);
                }
            }
        }

        // 更新索引
        for tag in &removed_tags {
            self.remove_from_indices(tag, relative_file_path);
        }

        // 如果文件没有标签了，移除文件记录
        if let Some(file_tags) = self.file_tags.get(relative_file_path) {
            if file_tags.is_empty() {
                self.file_tags.remove(relative_file_path);
            }
        }

        if !removed_tags.is_empty() {
            self.save_to_storage().await?;
            info!("从文件 {} 移除了 {} 个标签: {:?}", relative_file_path, removed_tags.len(), removed_tags);
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
        let query = query.trim();

        if query.is_empty() {
            return Ok(Vec::new());
        }

        // 解析并执行查询
        let result = self.parse_and_execute_query(query)?;
        let mut files: Vec<String> = result.into_iter().collect();
        files.sort();
        Ok(files)
    }

    /// 解析并执行查询表达式
    fn parse_and_execute_query(&self, query: &str) -> Result<std::collections::HashSet<String>> {
        // 处理 OR 操作（优先级最低）
        if query.contains(" OR ") {
            let parts: Vec<&str> = query.split(" OR ").map(|s| s.trim()).collect();
            let mut result = std::collections::HashSet::new();
            for part in parts {
                let part_result = self.parse_and_execute_query(part)?;
                result.extend(part_result);
            }
            return Ok(result);
        }

        // 处理 AND 操作
        if query.contains(" AND ") {
            let parts: Vec<&str> = query.split(" AND ").map(|s| s.trim()).collect();
            let mut result = None;
            for part in parts {
                let part_result = self.parse_and_execute_query(part)?;
                match result {
                    None => result = Some(part_result),
                    Some(ref mut current) => {
                        *current = current.intersection(&part_result).cloned().collect();
                    }
                }
            }
            return Ok(result.unwrap_or_default());
        }

        // 处理 NOT 操作
        if query.starts_with("NOT ") {
            let inner_query = &query[4..].trim();
            let inner_result = self.parse_and_execute_query(inner_query)?;
            let all_files: std::collections::HashSet<String> = self.file_tags.keys().cloned().collect();
            return Ok(all_files.difference(&inner_result).cloned().collect());
        }

        // 处理括号表达式
        if query.starts_with('(') && query.ends_with(')') {
            let inner_query = &query[1..query.len()-1];
            return self.parse_and_execute_query(inner_query);
        }

        // 处理通配符查询
        if query.contains('*') {
            return self.execute_wildcard_query(query);
        }

        // 单个标签查询
        Ok(self.tag_to_files
            .get(query)
            .map(|files| files.iter().cloned().collect())
            .unwrap_or_default())
    }

    /// 执行通配符查询
    fn execute_wildcard_query(&self, pattern: &str) -> Result<std::collections::HashSet<String>> {
        let mut result = std::collections::HashSet::new();

        // 简单的通配符实现：支持 * 匹配任意字符
        for tag in self.tag_to_files.keys() {
            if self.wildcard_match(pattern, tag) {
                if let Some(files) = self.tag_to_files.get(tag) {
                    result.extend(files.iter().cloned());
                }
            }
        }

        Ok(result)
    }

    /// 简单的通配符匹配实现
    fn wildcard_match(&self, pattern: &str, text: &str) -> bool {
        // 如果模式中没有通配符，直接比较
        if !pattern.contains('*') {
            return pattern == text;
        }

        // 将模式按 * 分割
        let parts: Vec<&str> = pattern.split('*').collect();

        // 如果只有一个部分，说明没有 *
        if parts.len() == 1 {
            return pattern == text;
        }

        let mut text_pos = 0;

        // 检查第一部分（如果不为空）
        if !parts[0].is_empty() {
            if !text.starts_with(parts[0]) {
                return false;
            }
            text_pos += parts[0].len();
        }

        // 检查最后一部分（如果不为空）
        if !parts[parts.len() - 1].is_empty() {
            if !text.ends_with(parts[parts.len() - 1]) {
                return false;
            }
        }

        // 检查中间部分
        for i in 1..parts.len() - 1 {
            if !parts[i].is_empty() {
                if let Some(pos) = text[text_pos..].find(parts[i]) {
                    text_pos += pos + parts[i].len();
                } else {
                    return false;
                }
            }
        }

        true
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
