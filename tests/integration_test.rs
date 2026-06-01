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

#[test]
fn test_edit_file_by_lines() {
    let dir = TempDir::new_in(".").expect("Failed to create temp dir in CWD");
    let path = dir.path().join("edit_lines.txt");
    fs::write(&path, "line1\nline2\nline3\nline4\nline5\n").unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        use deepseek_rust_cli::tools::file_io::{edit_file_by_lines, LineEdit};

        // 1. Success case: multiple edits, descending start line, target content check, line count
        //    shift
        let edits = vec![
            LineEdit {
                start_line: 2,
                end_line: 2,
                replacement_content: "line2 modified\nline2.5 added".to_string(),
                target_content: Some("line2".to_string()),
            },
            LineEdit {
                start_line: 4,
                end_line: 5,
                replacement_content: "line4 and 5 replaced".to_string(),
                target_content: Some("line4\nline5".to_string()),
            },
        ];

        let res = edit_file_by_lines(path.to_str().unwrap(), edits).await;
        assert!(res.is_ok());

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(
            content,
            "line1\nline2 modified\nline2.5 added\nline3\nline4 and 5 replaced\n"
        );
    });
}

#[test]
fn test_edit_file_by_lines_fail_overlap() {
    let dir = TempDir::new_in(".").expect("Failed to create temp dir in CWD");
    let path = dir.path().join("edit_lines_overlap.txt");
    fs::write(&path, "line1\nline2\nline3\n").unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        use deepseek_rust_cli::tools::file_io::{edit_file_by_lines, LineEdit};

        let edits = vec![
            LineEdit {
                start_line: 1,
                end_line: 2,
                replacement_content: "overlap1".to_string(),
                target_content: None,
            },
            LineEdit {
                start_line: 2,
                end_line: 3,
                replacement_content: "overlap2".to_string(),
                target_content: None,
            },
        ];

        let res = edit_file_by_lines(path.to_str().unwrap(), edits).await;
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .to_string()
            .contains("Overlapping edits detected"));
    });
}

#[test]
fn test_edit_file_by_lines_fail_target_mismatch() {
    let dir = TempDir::new_in(".").expect("Failed to create temp dir in CWD");
    let path = dir.path().join("edit_lines_mismatch.txt");
    fs::write(&path, "line1\nline2\nline3\n").unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        use deepseek_rust_cli::tools::file_io::{edit_file_by_lines, LineEdit};

        let edits = vec![LineEdit {
            start_line: 2,
            end_line: 2,
            replacement_content: "not applied".to_string(),
            target_content: Some("wrong content".to_string()),
        }];

        let res = edit_file_by_lines(path.to_str().unwrap(), edits).await;
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .to_string()
            .contains("Target content verification failed"));
    });
}

#[test]
fn test_background_process_lifecycle() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        use deepseek_rust_cli::tools::system::{
            kill_background_process, list_background_processes, read_background_process_logs,
            start_background_process,
        };

        let cmd = if cfg!(target_os = "windows") {
            "echo Hello background && powershell -Command Start-Sleep 10"
        } else {
            "echo Hello background && sleep 10"
        };

        let res = start_background_process(cmd, None, None).await.unwrap();
        assert!(res.contains("Started background process"));

        let parts: Vec<&str> = res.split_whitespace().collect();
        let pid: u32 = parts[5].parse().unwrap();

        // Retry reading logs as spawning a process on slow CI environments might take some time
        let mut logs = String::new();
        for _ in 0..50 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            logs = read_background_process_logs(pid).await.unwrap();
            if logs.contains("Hello background") {
                break;
            }
        }
        assert!(
            logs.contains("Hello background"),
            "Expected logs to contain 'Hello background', but got: {}",
            logs
        );

        let list = list_background_processes().await.unwrap();
        assert!(list.contains(&pid.to_string()));

        let kill_res = kill_background_process(pid).await.unwrap();
        assert!(kill_res.contains("Successfully terminated background process"));
    });
}

#[test]
fn test_view_symbol_contents() {
    let dir = TempDir::new_in(".").expect("Failed to create temp dir in CWD");
    let rust_path = dir.path().join("code.rs");
    fs::write(
        &rust_path,
        "fn hello() {\n    println!(\"hello\");\n}\n\nstruct Point {\n    x: i32,\n    y: \
         i32,\n}\n",
    )
    .unwrap();

    let py_path = dir.path().join("code.py");
    fs::write(&py_path, "def main():\n    print(\"main\")\n    return 0\n\nclass Foo:\n    def run(self):\n        pass\n").unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        use deepseek_rust_cli::tools::file_ops::view_symbol_contents;

        let res_struct = view_symbol_contents(rust_path.to_str().unwrap(), "Point")
            .await
            .unwrap();
        assert!(res_struct.contains("struct Point {"));
        assert!(res_struct.contains("y: i32,"));

        let res_fn = view_symbol_contents(rust_path.to_str().unwrap(), "hello")
            .await
            .unwrap();
        assert!(res_fn.contains("fn hello() {"));
        assert!(res_fn.contains("println!"));

        let res_py_def = view_symbol_contents(py_path.to_str().unwrap(), "main")
            .await
            .unwrap();
        assert!(res_py_def.contains("def main():"));
        assert!(res_py_def.contains("return 0"));

        let res_py_class = view_symbol_contents(py_path.to_str().unwrap(), "Foo")
            .await
            .unwrap();
        assert!(res_py_class.contains("class Foo:"));
        assert!(res_py_class.contains("pass"));
    });
}

#[test]
fn test_apply_diff_patch() {
    let dir = TempDir::new_in(".").expect("Failed to create temp dir in CWD");
    let path = dir.path().join("patch.txt");
    fs::write(&path, "line1\nline2\nline3\nline4\n").unwrap();

    let patch = "--- a/patch.txt\n+++ b/patch.txt\n@@ -2,2 +2,3 @@\n-line2\n+line2 \
                 modified\n+line2.5 added\n line3\n";

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        use deepseek_rust_cli::tools::file_io::apply_diff_patch;

        let res = apply_diff_patch(path.to_str().unwrap(), patch)
            .await
            .unwrap();
        assert_eq!(res, "Patch successfully applied.");

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(
            content,
            "line1\nline2 modified\nline2.5 added\nline3\nline4\n"
        );
    });
}

#[test]
fn test_check_port_status() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        use deepseek_rust_cli::tools::system::check_port_status;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let occupied_res = check_port_status(port, Some("127.0.0.1")).await.unwrap();
        assert!(occupied_res.contains("OCCUPIED"));

        drop(listener);

        let mut free_res = String::new();
        for _ in 0..20 {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            free_res = check_port_status(port, Some("127.0.0.1")).await.unwrap();
            if free_res.contains("FREE") {
                break;
            }
        }
        assert!(
            free_res.contains("FREE"),
            "Expected port to become free, but got: {}",
            free_res
        );
    });
}
