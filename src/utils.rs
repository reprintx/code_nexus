use crate::error::{CodeNexusError, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// 验证项目路径
pub fn validate_project_path(project_path: &str) -> Result<PathBuf> {
    if project_path.trim().is_empty() {
        return Err(CodeNexusError::ConfigError("项目路径不能为空".to_string()));
    }

    let path = Path::new(project_path);
    
    // 检查路径是否存在
    if !path.exists() {
        return Err(CodeNexusError::FileNotFound(format!(
            "项目路径不存在: {}",
            project_path
        )));
    }

    // 检查是否为目录
    if !path.is_dir() {
        return Err(CodeNexusError::ConfigError(format!(
            "项目路径必须是目录: {}",
            project_path
        )));
    }

    // 转换为绝对路径
    let absolute_path = path.canonicalize().map_err(|e| {
        CodeNexusError::FileSystemError(format!(
            "无法解析项目路径 {}: {}",
            project_path, e
        ))
    })?;

    debug!("项目路径验证成功: {:?}", absolute_path);
    Ok(absolute_path)
}

/// 验证文件路径（相对于项目根目录）
pub fn validate_file_path(project_path: &Path, file_path: &str) -> Result<PathBuf> {
    if file_path.trim().is_empty() {
        return Err(CodeNexusError::ConfigError("文件路径不能为空".to_string()));
    }

    // 构建完整的文件路径
    let full_path = project_path.join(file_path);

    // 检查文件是否存在
    if !full_path.exists() {
        return Err(CodeNexusError::FileNotFound(format!(
            "文件不存在: {} (完整路径: {:?})",
            file_path, full_path
        )));
    }

    // 检查是否为文件
    if !full_path.is_file() {
        return Err(CodeNexusError::ConfigError(format!(
            "路径必须指向文件而不是目录: {}",
            file_path
        )));
    }

    // 确保文件在项目目录内（安全检查）
    let canonical_full_path = full_path.canonicalize().map_err(|e| {
        CodeNexusError::FileSystemError(format!(
            "无法解析文件路径 {}: {}",
            file_path, e
        ))
    })?;

    let canonical_project_path = project_path.canonicalize().map_err(|e| {
        CodeNexusError::FileSystemError(format!(
            "无法解析项目路径 {:?}: {}",
            project_path, e
        ))
    })?;

    if !canonical_full_path.starts_with(&canonical_project_path) {
        warn!("安全警告: 文件路径超出项目范围: {:?}", canonical_full_path);
        return Err(CodeNexusError::ConfigError(format!(
            "文件路径必须在项目目录内: {}",
            file_path
        )));
    }

    debug!("文件路径验证成功: {:?}", canonical_full_path);
    Ok(canonical_full_path)
}

/// 获取数据存储目录路径
pub fn get_data_dir(project_path: &Path) -> PathBuf {
    project_path.join(".codenexus")
}

/// 规范化文件路径（转换为相对于项目根目录的路径）
pub fn normalize_file_path(project_path: &Path, file_path: &Path) -> Result<String> {
    let canonical_project = project_path.canonicalize().map_err(|e| {
        CodeNexusError::FileSystemError(format!(
            "无法解析项目路径 {:?}: {}",
            project_path, e
        ))
    })?;

    let canonical_file = file_path.canonicalize().map_err(|e| {
        CodeNexusError::FileSystemError(format!(
            "无法解析文件路径 {:?}: {}",
            file_path, e
        ))
    })?;

    let relative_path = canonical_file.strip_prefix(&canonical_project).map_err(|_| {
        CodeNexusError::ConfigError(format!(
            "文件路径不在项目目录内: {:?}",
            file_path
        ))
    })?;

    // 转换为字符串，使用正斜杠作为分隔符（跨平台兼容）
    let normalized = relative_path
        .to_string_lossy()
        .replace('\\', "/");

    Ok(normalized)
}

/// 创建项目错误信息
pub fn project_path_error(message: String) -> CodeNexusError {
    CodeNexusError::ConfigError(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_validate_project_path() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_str().unwrap();

        // 测试有效路径
        let result = validate_project_path(project_path);
        assert!(result.is_ok());

        // 测试空路径
        let result = validate_project_path("");
        assert!(result.is_err());

        // 测试不存在的路径
        let result = validate_project_path("/nonexistent/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        // 创建测试文件
        let test_file = project_path.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        // 测试有效文件路径
        let result = validate_file_path(project_path, "test.txt");
        assert!(result.is_ok());

        // 测试不存在的文件
        let result = validate_file_path(project_path, "nonexistent.txt");
        assert!(result.is_err());

        // 测试空路径
        let result = validate_file_path(project_path, "");
        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        // 创建测试文件
        let test_file = project_path.join("src").join("main.rs");
        fs::create_dir_all(test_file.parent().unwrap()).unwrap();
        fs::write(&test_file, "fn main() {}").unwrap();

        let result = normalize_file_path(project_path, &test_file);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "src/main.rs");
    }
}
