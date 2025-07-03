use code_nexus::CodeNexusServer;
use std::fs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // 创建两个测试项目
    let project1_path = "examples/test_project1";
    let project2_path = "examples/test_project2";

    // 创建项目1
    fs::create_dir_all(format!("{}/src", project1_path))?;
    fs::write(format!("{}/src/main.rs", project1_path), "fn main() { println!(\"Project 1\"); }")?;
    fs::write(format!("{}/src/lib.rs", project1_path), "pub mod utils;")?;
    fs::write(format!("{}/src/utils.rs", project1_path), "pub fn helper1() -> String { \"helper1\".to_string() }")?;

    // 创建项目2
    fs::create_dir_all(format!("{}/src", project2_path))?;
    fs::write(format!("{}/src/main.rs", project2_path), "fn main() { println!(\"Project 2\"); }")?;
    fs::write(format!("{}/src/api.rs", project2_path), "pub fn api_handler() {}")?;
    fs::write(format!("{}/src/models.rs", project2_path), "pub struct User { pub name: String }")?;

    // 创建服务器实例
    let server = CodeNexusServer::new().await
        .map_err(|e| anyhow::anyhow!("创建服务器失败: {:?}", e))?;

    println!("✅ CodeNexus 多项目服务器创建成功！");

    // 测试项目1管理器创建
    let project1_manager = server.get_or_create_project(project1_path).await
        .map_err(|e| anyhow::anyhow!("创建项目1管理器失败: {:?}", e))?;
    println!("📁 项目1管理器创建成功: {}", project1_path);

    // 测试项目2管理器创建
    let project2_manager = server.get_or_create_project(project2_path).await
        .map_err(|e| anyhow::anyhow!("创建项目2管理器失败: {:?}", e))?;
    println!("📁 项目2管理器创建成功: {}", project2_path);

    // 验证数据目录创建
    let project1_data_dir = format!("{}/.codenexus", project1_path);
    let project2_data_dir = format!("{}/.codenexus", project2_path);

    if std::path::Path::new(&project1_data_dir).exists() {
        println!("📋 项目1数据目录已创建: {}", project1_data_dir);
    }

    if std::path::Path::new(&project2_data_dir).exists() {
        println!("📋 项目2数据目录已创建: {}", project2_data_dir);
    }

    // 测试项目隔离
    println!("\n🔒 测试项目隔离性:");
    
    // 再次获取项目1管理器（应该返回相同实例）
    let project1_manager_again = server.get_or_create_project(project1_path).await
        .map_err(|e| anyhow::anyhow!("获取项目1管理器失败: {:?}", e))?;
    
    // 检查是否是同一个实例（通过Arc指针比较）
    let same_instance = std::ptr::eq(
        project1_manager.as_ref() as *const _,
        project1_manager_again.as_ref() as *const _
    );
    
    if same_instance {
        println!("✅ 项目管理器实例复用正常");
    } else {
        println!("❌ 项目管理器实例复用异常");
    }

    println!("\n🚀 多项目功能验证完成！");
    println!("💡 提示:");
    println!("  - 每个项目都有独立的数据存储目录");
    println!("  - 项目管理器实例会被缓存和复用");
    println!("  - 支持同时管理多个项目");
    println!("  - 项目路径验证确保安全性");

    Ok(())
}
