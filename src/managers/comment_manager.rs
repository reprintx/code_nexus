use crate::error::{CodeNexusError, Result};
use crate::storage::{JsonStorage, CommentsData};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info};

/// 注释管理器
#[derive(Debug)]
pub struct CommentManager {
    storage: JsonStorage,
    // 内存数据
    file_comments: HashMap<String, String>,
}

impl CommentManager {
    /// 创建新的注释管理器
    pub fn new(storage: JsonStorage) -> Self {
        Self {
            storage,
            file_comments: HashMap::new(),
        }
    }

    /// 初始化管理器，加载数据到内存
    pub async fn initialize(&mut self) -> Result<()> {
        let data = self.storage.load_comments().await?;
        self.file_comments = data.file_comments;
        info!("注释管理器初始化完成，加载了 {} 个文件的注释", self.file_comments.len());
        Ok(())
    }

    /// 验证文件路径（使用绝对路径）
    fn validate_file_path(&self, absolute_file_path: &Path) -> Result<()> {
        if !absolute_file_path.exists() {
            return Err(CodeNexusError::FileNotFound(absolute_file_path.to_string_lossy().to_string()));
        }
        Ok(())
    }

    /// 验证注释内容
    fn validate_comment(&self, comment: &str) -> Result<()> {
        if comment.trim().is_empty() {
            return Err(CodeNexusError::ConfigError("注释内容不能为空".to_string()));
        }
        Ok(())
    }

    /// 为文件添加注释
    pub async fn add_comment(&mut self, absolute_file_path: &Path, relative_file_path: &str, comment: &str) -> Result<()> {
        // 验证输入
        self.validate_file_path(absolute_file_path)?;
        self.validate_comment(comment)?;

        // 检查是否已存在注释（使用相对路径）
        if self.file_comments.contains_key(relative_file_path) {
            return Err(CodeNexusError::ConfigError(
                format!("文件 {} 已存在注释，请使用 update_comment 更新", relative_file_path)
            ));
        }

        // 添加注释（使用相对路径存储）
        self.file_comments.insert(relative_file_path.to_string(), comment.to_string());

        // 保存到存储
        self.save_to_storage().await?;
        info!("为文件 {} 添加了注释", relative_file_path);

        Ok(())
    }

    /// 更新文件注释
    pub async fn update_comment(&mut self, absolute_file_path: &Path, relative_file_path: &str, comment: &str) -> Result<()> {
        // 验证输入
        self.validate_file_path(absolute_file_path)?;
        self.validate_comment(comment)?;

        // 更新注释（使用相对路径存储）
        let old_comment = self.file_comments.insert(relative_file_path.to_string(), comment.to_string());

        // 保存到存储
        self.save_to_storage().await?;

        if old_comment.is_some() {
            info!("更新了文件 {} 的注释", relative_file_path);
        } else {
            info!("为文件 {} 添加了注释", relative_file_path);
        }

        Ok(())
    }

    /// 获取文件注释
    pub fn get_comment(&self, file_path: &str) -> Option<String> {
        self.file_comments.get(file_path).cloned()
    }

    /// 批量获取文件注释
    pub fn get_comments(&self, file_paths: &[String]) -> HashMap<String, String> {
        let mut result = HashMap::new();
        for file_path in file_paths {
            if let Some(comment) = self.file_comments.get(file_path) {
                result.insert(file_path.clone(), comment.clone());
            }
        }
        result
    }

    /// 获取所有注释
    pub fn get_all_comments(&self) -> &HashMap<String, String> {
        &self.file_comments
    }

    /// 删除文件注释
    pub async fn delete_comment(&mut self, file_path: &str) -> Result<()> {
        if let Some(_) = self.file_comments.remove(file_path) {
            self.save_to_storage().await?;
            info!("删除了文件 {} 的注释", file_path);
            Ok(())
        } else {
            Err(CodeNexusError::FileNotFound(
                format!("文件 {} 没有注释", file_path)
            ))
        }
    }

    /// 检查文件是否有注释
    pub fn has_comment(&self, file_path: &str) -> bool {
        self.file_comments.contains_key(file_path)
    }

    /// 获取有注释的文件列表
    pub fn get_commented_files(&self) -> Vec<String> {
        let mut files: Vec<String> = self.file_comments.keys().cloned().collect();
        files.sort();
        files
    }

    /// 搜索注释内容（简单的关键词搜索）
    pub fn search_comments(&self, keyword: &str) -> Vec<(String, String)> {
        let keyword_lower = keyword.to_lowercase();
        let mut results = Vec::new();

        for (file_path, comment) in &self.file_comments {
            if comment.to_lowercase().contains(&keyword_lower) {
                results.push((file_path.clone(), comment.clone()));
            }
        }

        results.sort_by(|a, b| a.0.cmp(&b.0));
        results
    }

    /// 获取注释统计信息
    pub fn get_stats(&self) -> (usize, usize) {
        let total_comments = self.file_comments.len();
        let total_chars: usize = self.file_comments.values().map(|c| c.len()).sum();
        (total_comments, total_chars)
    }

    /// 清理不存在文件的注释
    pub async fn cleanup_invalid_comments(&mut self) -> Result<usize> {
        let mut removed_count = 0;
        let mut files_to_remove = Vec::new();

        for file_path in self.file_comments.keys() {
            if !Path::new(file_path).exists() {
                files_to_remove.push(file_path.clone());
            }
        }

        for file_path in files_to_remove {
            self.file_comments.remove(&file_path);
            removed_count += 1;
            debug!("清理了不存在文件的注释: {}", file_path);
        }

        if removed_count > 0 {
            self.save_to_storage().await?;
            info!("清理了 {} 个无效注释", removed_count);
        }

        Ok(removed_count)
    }

    /// 导出注释数据
    pub fn export_comments(&self) -> HashMap<String, String> {
        self.file_comments.clone()
    }

    /// 导入注释数据
    pub async fn import_comments(&mut self, comments: HashMap<String, String>) -> Result<usize> {
        let mut imported_count = 0;

        for (file_path, comment) in comments {
            // 验证文件路径和注释内容
            if Path::new(&file_path).exists() && !comment.trim().is_empty() {
                self.file_comments.insert(file_path, comment);
                imported_count += 1;
            }
        }

        if imported_count > 0 {
            self.save_to_storage().await?;
            info!("导入了 {} 个注释", imported_count);
        }

        Ok(imported_count)
    }

    /// 保存数据到存储
    async fn save_to_storage(&self) -> Result<()> {
        let data = CommentsData {
            file_comments: self.file_comments.clone(),
        };

        self.storage.save_comments(&data).await
    }
}
