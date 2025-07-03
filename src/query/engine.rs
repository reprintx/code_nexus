use crate::error::{CodeNexusError, Result};
use crate::managers::{TagManager, CommentManager, RelationManager};
use crate::models::{FileInfo, QueryResult, SystemStatus, TagStats};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

/// 查询引擎
#[derive(Debug)]
pub struct QueryEngine {
    tag_manager: Arc<Mutex<TagManager>>,
    comment_manager: Arc<Mutex<CommentManager>>,
    relation_manager: Arc<Mutex<RelationManager>>,
}

impl QueryEngine {
    /// 创建新的查询引擎
    pub fn new(
        tag_manager: Arc<Mutex<TagManager>>,
        comment_manager: Arc<Mutex<CommentManager>>,
        relation_manager: Arc<Mutex<RelationManager>>,
    ) -> Self {
        Self {
            tag_manager,
            comment_manager,
            relation_manager,
        }
    }

    /// 执行标签查询
    pub async fn execute_tag_query(&self, query: &str) -> Result<QueryResult> {
        let tag_manager = self.tag_manager.lock().await;
        let files = tag_manager.query_files_by_tags(query)?;
        
        Ok(QueryResult {
            total: files.len(),
            files,
        })
    }

    /// 获取文件完整信息
    pub async fn get_file_info(&self, file_path: &str) -> Result<FileInfo> {
        // 并行获取各种信息
        let (tags, comment, relations, incoming_relations) = tokio::join!(
            async {
                let tag_manager = self.tag_manager.lock().await;
                tag_manager.get_file_tags(file_path)
            },
            async {
                let comment_manager = self.comment_manager.lock().await;
                comment_manager.get_comment(file_path)
            },
            async {
                let relation_manager = self.relation_manager.lock().await;
                relation_manager.get_file_relations(file_path)
            },
            async {
                let relation_manager = self.relation_manager.lock().await;
                relation_manager.get_incoming_relations(file_path)
            }
        );

        Ok(FileInfo {
            path: file_path.to_string(),
            tags,
            comment,
            relations,
            incoming_relations,
        })
    }

    /// 复合查询：结合标签和关联关系
    pub async fn execute_complex_query(
        &self,
        tag_query: Option<&str>,
        relation_keyword: Option<&str>,
    ) -> Result<QueryResult> {
        let mut result_files = Vec::new();

        // 如果有标签查询
        if let Some(query) = tag_query {
            let tag_manager = self.tag_manager.lock().await;
            result_files = tag_manager.query_files_by_tags(query)?;
        }

        // 如果有关联关系关键词搜索
        if let Some(keyword) = relation_keyword {
            let relation_manager = self.relation_manager.lock().await;
            let relation_results = relation_manager.query_relations_by_description(keyword);
            
            let relation_files: Vec<String> = relation_results
                .into_iter()
                .map(|(from_file, _)| from_file)
                .collect();

            if result_files.is_empty() {
                result_files = relation_files;
            } else {
                // 求交集
                result_files.retain(|file| relation_files.contains(file));
            }
        }

        // 去重并排序
        result_files.sort();
        result_files.dedup();

        Ok(QueryResult {
            total: result_files.len(),
            files: result_files,
        })
    }

    /// 获取系统状态
    pub async fn get_system_status(&self) -> Result<SystemStatus> {
        let (tag_stats, comment_stats, relation_stats) = tokio::join!(
            async {
                let tag_manager = self.tag_manager.lock().await;
                tag_manager.get_stats()
            },
            async {
                let comment_manager = self.comment_manager.lock().await;
                comment_manager.get_stats()
            },
            async {
                let relation_manager = self.relation_manager.lock().await;
                relation_manager.get_stats()
            }
        );

        let tag_manager = self.tag_manager.lock().await;
        let all_tags = tag_manager.get_all_tags();

        let tag_stats_info = TagStats {
            tag_types: all_tags,
            total_files: tag_stats.0,
            total_tags: tag_stats.1,
        };

        Ok(SystemStatus {
            total_files: tag_stats.0.max(comment_stats.0).max(relation_stats.0),
            tagged_files: tag_stats.0,
            commented_files: comment_stats.0,
            total_relations: relation_stats.1,
            tag_stats: tag_stats_info,
        })
    }

    /// 搜索文件（综合搜索）
    pub async fn search_files(&self, keyword: &str) -> Result<Vec<FileInfo>> {
        let mut results = Vec::new();
        let mut file_set = std::collections::HashSet::new();

        // 搜索注释
        let comment_manager = self.comment_manager.lock().await;
        let comment_results = comment_manager.search_comments(keyword);
        
        for (file_path, _) in comment_results {
            file_set.insert(file_path);
        }
        drop(comment_manager);

        // 搜索关联关系描述
        let relation_manager = self.relation_manager.lock().await;
        let relation_results = relation_manager.query_relations_by_description(keyword);
        
        for (file_path, _) in relation_results {
            file_set.insert(file_path);
        }
        drop(relation_manager);

        // 获取每个文件的完整信息
        for file_path in file_set {
            if let Ok(file_info) = self.get_file_info(&file_path).await {
                results.push(file_info);
            }
        }

        // 按文件路径排序
        results.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(results)
    }

    /// 获取相关文件推荐
    pub async fn get_related_files(&self, file_path: &str, max_results: usize) -> Result<Vec<String>> {
        let mut related_files = std::collections::HashSet::new();

        // 基于标签的相关性
        let tag_manager = self.tag_manager.lock().await;
        let file_tags = tag_manager.get_file_tags(file_path);
        
        for tag in &file_tags {
            let tag_files = tag_manager.query_files_by_tags(tag)?;
            for tag_file in tag_files {
                if tag_file != file_path {
                    related_files.insert(tag_file);
                }
            }
        }
        drop(tag_manager);

        // 基于关联关系的相关性
        let relation_manager = self.relation_manager.lock().await;
        let outgoing_relations = relation_manager.get_file_relations(file_path);
        let incoming_relations = relation_manager.get_incoming_relations(file_path);

        for relation in outgoing_relations {
            related_files.insert(relation.target);
        }

        for relation in incoming_relations {
            related_files.insert(relation.target);
        }
        drop(relation_manager);

        // 转换为向量并限制结果数量
        let mut result: Vec<String> = related_files.into_iter().collect();
        result.sort();
        result.truncate(max_results);

        Ok(result)
    }

    /// 批量获取文件信息
    pub async fn get_batch_file_info(&self, file_paths: &[String]) -> Result<Vec<FileInfo>> {
        let mut results = Vec::new();

        for file_path in file_paths {
            match self.get_file_info(file_path).await {
                Ok(file_info) => results.push(file_info),
                Err(e) => {
                    debug!("获取文件信息失败 {}: {}", file_path, e);
                    // 继续处理其他文件，不中断整个批处理
                }
            }
        }

        Ok(results)
    }

    /// 验证查询语法
    pub fn validate_query_syntax(&self, query: &str) -> Result<()> {
        if query.trim().is_empty() {
            return Err(CodeNexusError::InvalidQuerySyntax("查询不能为空".to_string()));
        }

        // 简单的语法验证
        let query = query.trim();
        
        // 检查 AND 操作符
        if query.contains(" AND ") {
            let parts: Vec<&str> = query.split(" AND ").collect();
            for part in parts {
                if part.trim().is_empty() {
                    return Err(CodeNexusError::InvalidQuerySyntax(
                        "AND 操作符前后不能为空".to_string()
                    ));
                }
            }
        }

        // 检查标签格式（如果包含冒号）
        if query.contains(':') && !query.contains(' ') {
            // 单个标签格式检查
            if query.split(':').count() != 2 {
                return Err(CodeNexusError::InvalidQuerySyntax(
                    "标签格式应为 type:value".to_string()
                ));
            }
        }

        Ok(())
    }

    /// 获取查询建议
    pub async fn get_query_suggestions(&self, partial_query: &str) -> Result<Vec<String>> {
        let mut suggestions = Vec::new();

        if partial_query.is_empty() {
            return Ok(suggestions);
        }

        let tag_manager = self.tag_manager.lock().await;
        let all_tags = tag_manager.get_all_tags();

        // 基于标签类型的建议
        for (tag_type, tag_values) in all_tags {
            if tag_type.starts_with(partial_query) {
                for value in tag_values {
                    suggestions.push(format!("{}:{}", tag_type, value));
                }
            } else {
                // 基于标签值的建议
                for value in tag_values {
                    let full_tag = format!("{}:{}", tag_type, value);
                    if full_tag.contains(partial_query) {
                        suggestions.push(full_tag);
                    }
                }
            }
        }

        suggestions.sort();
        suggestions.truncate(10); // 限制建议数量

        Ok(suggestions)
    }
}
