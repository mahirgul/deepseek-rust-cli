pub mod base;
pub mod code_ops;
pub mod file_io;
pub mod file_ops;
pub mod git_ops;
pub mod github_ops;
pub mod schemas;
pub mod system;
pub mod web_ops;

// Tool implementations
pub mod file;
pub mod git_tools;
pub mod github_tools;
pub mod system_tools;
pub mod web_tools;

use crate::tools::base::Tool;

pub fn get_all_tools() -> Vec<Box<dyn Tool>> {
    vec![
        // File Tools
        Box::new(file::read_write::ReadFileTool),
        Box::new(file::read_write::WriteFileTool),
        Box::new(file::read_write::ReplaceTextTool),
        Box::new(file::ops::DeleteFileTool),
        Box::new(file::ops::RenameFileTool),
        Box::new(file::ops::CopyFileTool),
        Box::new(file::ops::CopyDirectoryTool),
        Box::new(file::ops::CreateDirectoryTool),
        Box::new(file::ops::FileExistsTool),
        Box::new(file::ops::GetFileInfoTool),
        Box::new(file::navigation::ListDirectoryTool),
        Box::new(file::navigation::TreeViewTool),
        Box::new(file::diff::DiffFilesTool),
        Box::new(file::ops::HashFileTool),
        Box::new(file::ops::CountLinesTool),
        Box::new(file::ops::SearchFilesTool),
        Box::new(file::ops::BulkRenameTool),
        // System Tools
        Box::new(system_tools::ShellTool),
        Box::new(system_tools::SystemInfoTool),
        Box::new(system_tools::StartBackgroundProcessTool),
        Box::new(system_tools::ReadBackgroundProcessLogsTool),
        Box::new(system_tools::KillBackgroundProcessTool),
        Box::new(system_tools::ListBackgroundProcessesTool),
        Box::new(system_tools::CheckPortStatusTool),
        // Web & Code Tools
        Box::new(web_tools::RunPythonTool),
        Box::new(web_tools::FetchUrlTool),
        Box::new(web_tools::GetEnvVarTool),
        // New Advanced Tools
        Box::new(file::read_write::RegexReplaceTool),
        Box::new(file::read_write::JsonUpdateValueTool),
        Box::new(file::read_write::EditFileByLinesTool),
        Box::new(file::read_write::ApplyDiffPatchTool),
        Box::new(file::ops::ListSymbolsTool),
        Box::new(file::ops::ViewSymbolContentsTool),
        Box::new(file::refactor::MoveCodeBlockTool),
        Box::new(file::refactor::SplitFileTool),
        Box::new(file::refactor::CleanupFileTool),
        Box::new(file::analysis::ProjectSummaryTool),
        Box::new(file::analysis::ListTodoTasksTool),
        Box::new(file::refactor::ProjectCheckpointTool),
        Box::new(file::refactor::RestoreCheckpointTool),
        Box::new(file::refactor::ProjectWideReplaceTool),
        Box::new(web_tools::ScreenshotWebappTool),
        Box::new(web_tools::WebSearchDuckDuckGoTool),
        // Git Tools
        Box::new(git_tools::GitStatusTool),
        Box::new(git_tools::GitDiffTool),
        Box::new(git_tools::GitLogTool),
        Box::new(git_tools::GitBranchTool),
        Box::new(git_tools::GitAddTool),
        Box::new(git_tools::GitCommitTool),
        Box::new(git_tools::GitPushTool),
        Box::new(git_tools::GitPullTool),
        Box::new(git_tools::GitCheckoutTool),
        Box::new(git_tools::GitCloneTool),
        Box::new(git_tools::GitRemoteListTool),
        Box::new(git_tools::GitStashTool),
        // Github Tools
        Box::new(github_tools::GithubRepoInfoTool),
        Box::new(github_tools::GithubRepoListIssuesTool),
        Box::new(github_tools::GithubIssueCreateTool),
        Box::new(github_tools::GithubIssueUpdateTool),
        Box::new(github_tools::GithubPrListTool),
        Box::new(github_tools::GithubPrCreateTool),
        Box::new(github_tools::GithubPrInfoTool),
        Box::new(github_tools::GithubPrMergeTool),
        Box::new(github_tools::GithubSearchCodeTool),
        Box::new(github_tools::GithubSearchReposTool),
        Box::new(github_tools::GithubGetFileTool),
        Box::new(github_tools::GithubWorkflowListTool),
        Box::new(github_tools::GithubWorkflowRunsTool),
    ]
}
