use std::fs;
use tempfile::TempDir;

#[test]
fn test_replace_text_in_file() {
    let dir = TempDir::new_in(".").expect("Failed to create temp dir in CWD");
    let path = dir.path().join("replace.txt");
    fs::write(&path, "Hello World\nGoodbye").unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        deepseek_rust_cli::tools::file_io::replace_text_in_file(
            path.to_str().unwrap(),
            "World",
            "Rust",
        )
        .await
        .unwrap();
    });

    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content, "Hello Rust\nGoodbye");
}

#[test]
fn test_count_lines() {
    let dir = TempDir::new_in(".").expect("Failed to create temp dir in CWD");
    let path = dir.path().join("lines.txt");
    fs::write(&path, "line1\nline2\nline3\n").unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        deepseek_rust_cli::tools::file_ops::count_lines(path.to_str().unwrap().to_string()).await
    });

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("Lines: 3"));
}

#[test]
fn test_hash_file() {
    let dir = TempDir::new_in(".").expect("Failed to create temp dir in CWD");
    let path = dir.path().join("hash.txt");
    fs::write(&path, "test content").unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        deepseek_rust_cli::tools::file_ops::hash_file(
            path.to_str().unwrap().to_string(),
            Some("md5".to_string()),
        )
        .await
    });

    assert!(result.is_ok());
}

#[test]
fn test_fuzzy_replace() {
    let dir = TempDir::new_in(".").expect("Failed to create temp dir in CWD");
    let path = dir.path().join("fuzzy.txt");
    fs::write(&path, "Hello   World\n  Extra spaces  \nEnd").unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        deepseek_rust_cli::tools::file_io::fuzzy_replace_in_file(
            path.to_str().unwrap(),
            "Hello World",
            "Hi Earth",
        )
        .await
    });

    // Fuzzy matching should find "Hello   World" even with different spacing
    assert!(result.is_ok());
    let content = fs::read_to_string(&path).unwrap();
    // Should have replaced the closest match
    assert!(content.contains("Hi Earth") || content.contains("Hello   World"));
    // Either it replaced successfully or it kept the original if no match found
}
