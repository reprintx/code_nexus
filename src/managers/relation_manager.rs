use crate::error::{CodeNexusError, Result};
use crate::models::Relation;
use crate::storage::{JsonStorage, RelationsData};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info};

/// 关联关系管理器
#[derive(Debug)]
pub struct RelationManager {
    storage: JsonStorage,
    // 内存数据
    file_relations: HashMap<String, Vec<Relation>>,
    // 反向索引：目标文件 -> 指向它的关联关系
    incoming_relations: HashMap<String, Vec<(String, String)>>, // target -> [(from_file, description)]
}

impl RelationManager {
    /// 创建新的关联关系管理器
    pub fn new(storage: JsonStorage) -> Self {
        Self {
            storage,
            file_relations: HashMap::new(),
            incoming_relations: HashMap::new(),
        }
    }

    /// 初始化管理器，加载数据到内存
    pub async fn initialize(&mut self) -> Result<()> {
        let data = self.storage.load_relations().await?;
        self.file_relations = data.file_relations;
        self.build_incoming_index();
        info!("关联关系管理器初始化完成，加载了 {} 个文件的关联关系", self.file_relations.len());
        Ok(())
    }

    /// 构建反向索引
    fn build_incoming_index(&mut self) {
        self.incoming_relations.clear();

        for (from_file, relations) in &self.file_relations {
            for relation in relations {
                self.incoming_relations
                    .entry(relation.target.clone())
                    .or_default()
                    .push((from_file.clone(), relation.description.clone()));
            }
        }
    }

    /// 验证文件路径（使用绝对路径）
    fn validate_file_path(&self, absolute_file_path: &Path) -> Result<()> {
        if !absolute_file_path.exists() {
            return Err(CodeNexusError::FileNotFound(absolute_file_path.to_string_lossy().to_string()));
        }
        Ok(())
    }

    /// 验证关联描述
    fn validate_description(&self, description: &str) -> Result<()> {
        if description.trim().is_empty() {
            return Err(CodeNexusError::ConfigError("关联描述不能为空".to_string()));
        }
        Ok(())
    }

    /// 添加文件关联关系
    pub async fn add_relation(&mut self,
                              absolute_from_file: &Path, relative_from_file: &str,
                              absolute_to_file: &Path, relative_to_file: &str,
                              description: &str) -> Result<()> {
        // 验证输入
        self.validate_file_path(absolute_from_file)?;
        self.validate_file_path(absolute_to_file)?;
        self.validate_description(description)?;

        // 检查是否已存在相同的关联关系（使用相对路径）
        if let Some(relations) = self.file_relations.get(relative_from_file) {
            for relation in relations {
                if relation.target == relative_to_file {
                    return Err(CodeNexusError::RelationAlreadyExists {
                        from: relative_from_file.to_string(),
                        to: relative_to_file.to_string(),
                    });
                }
            }
        }

        // 添加关联关系（使用相对路径存储）
        let new_relation = Relation {
            target: relative_to_file.to_string(),
            description: description.to_string(),
        };

        self.file_relations
            .entry(relative_from_file.to_string())
            .or_default()
            .push(new_relation);

        // 更新反向索引
        self.incoming_relations
            .entry(relative_to_file.to_string())
            .or_default()
            .push((relative_from_file.to_string(), description.to_string()));

        // 保存到存储
        self.save_to_storage().await?;
        info!("添加了关联关系: {} -> {} ({})", relative_from_file, relative_to_file, description);

        Ok(())
    }

    /// 移除文件关联关系
    pub async fn remove_relation(&mut self,
                                 absolute_from_file: &Path, relative_from_file: &str,
                                 absolute_to_file: &Path, relative_to_file: &str) -> Result<()> {
        // 验证文件路径
        self.validate_file_path(absolute_from_file)?;
        self.validate_file_path(absolute_to_file)?;

        // 检查关联关系是否存在（使用相对路径）
        let relations = self.file_relations.get_mut(relative_from_file)
            .ok_or_else(|| CodeNexusError::RelationNotFound {
                from: relative_from_file.to_string(),
                to: relative_to_file.to_string(),
            })?;

        // 查找并移除关联关系
        let initial_len = relations.len();
        relations.retain(|relation| relation.target != relative_to_file);

        if relations.len() == initial_len {
            return Err(CodeNexusError::RelationNotFound {
                from: relative_from_file.to_string(),
                to: relative_to_file.to_string(),
            });
        }

        // 如果文件没有关联关系了，移除文件记录
        if relations.is_empty() {
            self.file_relations.remove(relative_from_file);
        }

        // 更新反向索引
        if let Some(incoming) = self.incoming_relations.get_mut(relative_to_file) {
            incoming.retain(|(from, _)| from != relative_from_file);
            if incoming.is_empty() {
                self.incoming_relations.remove(relative_to_file);
            }
        }

        // 保存到存储
        self.save_to_storage().await?;
        info!("移除了关联关系: {} -> {}", relative_from_file, relative_to_file);

        Ok(())
    }

    /// 获取文件的出向关联关系
    pub fn get_file_relations(&self, file_path: &str) -> Vec<Relation> {
        self.file_relations
            .get(file_path)
            .cloned()
            .unwrap_or_default()
    }

    /// 获取文件的入向关联关系
    pub fn get_incoming_relations(&self, file_path: &str) -> Vec<Relation> {
        self.incoming_relations
            .get(file_path)
            .map(|incoming| {
                incoming
                    .iter()
                    .map(|(from_file, description)| Relation {
                        target: from_file.clone(),
                        description: description.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 根据描述搜索关联关系
    pub fn query_relations_by_description(&self, keyword: &str) -> Vec<(String, Relation)> {
        let keyword_lower = keyword.to_lowercase();
        let mut results = Vec::new();

        for (from_file, relations) in &self.file_relations {
            for relation in relations {
                if relation.description.to_lowercase().contains(&keyword_lower) {
                    results.push((from_file.clone(), relation.clone()));
                }
            }
        }

        results.sort_by(|a, b| a.0.cmp(&b.0));
        results
    }

    /// 获取所有关联关系
    pub fn get_all_relations(&self) -> &HashMap<String, Vec<Relation>> {
        &self.file_relations
    }

    /// 检查两个文件是否有关联关系
    pub fn has_relation(&self, from_file: &str, to_file: &str) -> bool {
        if let Some(relations) = self.file_relations.get(from_file) {
            relations.iter().any(|relation| relation.target == to_file)
        } else {
            false
        }
    }

    /// 获取有关联关系的文件列表
    pub fn get_related_files(&self) -> Vec<String> {
        let mut files: Vec<String> = self.file_relations.keys().cloned().collect();
        files.sort();
        files
    }

    /// 获取关联关系统计信息
    pub fn get_stats(&self) -> (usize, usize, usize) {
        let total_files_with_relations = self.file_relations.len();
        let total_relations: usize = self.file_relations.values().map(|relations| relations.len()).sum();
        let total_incoming_files = self.incoming_relations.len();
        (total_files_with_relations, total_relations, total_incoming_files)
    }

    /// 获取文件的关联图谱（递归查找）
    pub fn get_relation_graph(&self, file_path: &str, max_depth: usize) -> HashMap<String, Vec<Relation>> {
        let mut graph = HashMap::new();
        let mut visited = std::collections::HashSet::new();
        self.build_relation_graph(file_path, max_depth, 0, &mut graph, &mut visited);
        graph
    }

    /// 递归构建关联图谱
    fn build_relation_graph(
        &self,
        file_path: &str,
        max_depth: usize,
        current_depth: usize,
        graph: &mut HashMap<String, Vec<Relation>>,
        visited: &mut std::collections::HashSet<String>,
    ) {
        if current_depth >= max_depth || visited.contains(file_path) {
            return;
        }

        visited.insert(file_path.to_string());

        if let Some(relations) = self.file_relations.get(file_path) {
            graph.insert(file_path.to_string(), relations.clone());

            // 递归查找关联文件的关联关系
            for relation in relations {
                self.build_relation_graph(
                    &relation.target,
                    max_depth,
                    current_depth + 1,
                    graph,
                    visited,
                );
            }
        }
    }

    /// 清理不存在文件的关联关系
    pub async fn cleanup_invalid_relations(&mut self) -> Result<usize> {
        let mut removed_count = 0;
        let mut files_to_remove = Vec::new();
        let mut relations_to_update = Vec::new();

        // 检查源文件是否存在
        for file_path in self.file_relations.keys() {
            if !Path::new(file_path).exists() {
                files_to_remove.push(file_path.clone());
            }
        }

        // 检查目标文件是否存在
        for (from_file, relations) in &self.file_relations {
            let mut valid_relations = Vec::new();
            for relation in relations {
                if Path::new(&relation.target).exists() {
                    valid_relations.push(relation.clone());
                } else {
                    removed_count += 1;
                    debug!("清理了指向不存在文件的关联: {} -> {}", from_file, relation.target);
                }
            }
            if valid_relations.len() != relations.len() {
                relations_to_update.push((from_file.clone(), valid_relations));
            }
        }

        // 移除不存在的源文件
        for file_path in files_to_remove {
            if let Some(relations) = self.file_relations.remove(&file_path) {
                removed_count += relations.len();
                debug!("清理了不存在文件的所有关联: {}", file_path);
            }
        }

        // 更新有效的关联关系
        for (from_file, valid_relations) in relations_to_update {
            if valid_relations.is_empty() {
                self.file_relations.remove(&from_file);
            } else {
                self.file_relations.insert(from_file, valid_relations);
            }
        }

        if removed_count > 0 {
            self.build_incoming_index(); // 重建反向索引
            self.save_to_storage().await?;
            info!("清理了 {} 个无效关联关系", removed_count);
        }

        Ok(removed_count)
    }

    /// 保存数据到存储
    async fn save_to_storage(&self) -> Result<()> {
        let data = RelationsData {
            file_relations: self.file_relations.clone(),
        };

        self.storage.save_relations(&data).await
    }
}
