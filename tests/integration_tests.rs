use code_nexus::CodeNexusServer;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_server_initialization() {
    // 创建临时目录用于测试
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();
    
    // 设置工作目录
    std::env::set_current_dir(&temp_dir).unwrap();
    
    // 创建测试文件
    fs::write("test.rs", "fn main() { println!(\"Hello\"); }").unwrap();
    
    // 初始化服务器
    let server = CodeNexusServer::new().await;
    assert!(server.is_ok(), "服务器初始化应该成功");
    
    // 检查数据目录是否创建
    assert!(std::path::Path::new(".codenexus").exists(), "数据目录应该被创建");
    assert!(std::path::Path::new(".codenexus/tags.json").exists(), "标签文件应该被创建");
    assert!(std::path::Path::new(".codenexus/comments.json").exists(), "注释文件应该被创建");
    assert!(std::path::Path::new(".codenexus/relations.json").exists(), "关联关系文件应该被创建");
}
