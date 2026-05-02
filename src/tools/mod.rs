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
pub mod file_tools;
pub mod git_tools;
pub mod github_tools;
pub mod system_tools;
pub mod web_tools;

use crate::tools::base::Tool;

pub fn get_all_tools() -> Vec<Box<dyn Tool>> {
    vec![
        // File Tools
        Box::new(file_tools::ReadFileTool),
        Box::new(file_tools::WriteFileTool),
        Box::new(file_tools::ReplaceTextTool),
        Box::new(file_tools::DeleteFileTool),
        Box::new(file_tools::RenameFileTool),
        Box::new(file_tools::ListDirectoryTool),
        Box::new(file_tools::TreeViewTool),
        Box::new(file_tools::DiffFilesTool),
        Box::new(file_tools::HashFileTool),
        Box::new(file_tools::CountLinesTool),
        // System Tools
        Box::new(system_tools::ShellTool),
        Box::new(system_tools::SystemInfoTool),
        // Web & Code Tools
        Box::new(web_tools::RunPythonTool),
        Box::new(web_tools::FetchUrlTool),
        Box::new(web_tools::GetEnvVarTool),
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
