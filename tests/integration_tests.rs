use code_nexus::CodeNexusServer;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_server_initialization() {
    // 创建临时目录用于测试
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path().to_str().unwrap();

    // 创建测试文件
    let test_file = temp_dir.path().join("test.rs");
    fs::write(&test_file, "fn main() { println!(\"Hello\"); }").unwrap();

    // 初始化服务器
    let server = CodeNexusServer::new().await;
    assert!(server.is_ok(), "服务器初始化应该成功");

    // 测试项目管理器创建（通过调用一个需要项目路径的方法）
    let server = server.unwrap();

    // 这会触发项目管理器的创建
    let project_manager = server.get_or_create_project(project_path).await;
    assert!(project_manager.is_ok(), "项目管理器创建应该成功");

    // 检查数据目录是否创建
    let data_dir = temp_dir.path().join(".codenexus");
    assert!(data_dir.exists(), "数据目录应该被创建");
    assert!(data_dir.join("tags.json").exists(), "标签文件应该被创建");
    assert!(data_dir.join("comments.json").exists(), "注释文件应该被创建");
    assert!(data_dir.join("relations.json").exists(), "关联关系文件应该被创建");
}
