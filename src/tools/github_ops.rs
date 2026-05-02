use anyhow::Result;
use std::env;

// ─── GitHub API Client ──────────────────────────────────────────────

fn get_github_token() -> Result<String> {
    env::var("GITHUB_TOKEN")
        .or_else(|_| env::var("GH_TOKEN"))
        .map_err(|_| anyhow::anyhow!("GITHUB_TOKEN not set. Please export GITHUB_TOKEN=your_token"))
}

fn create_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("deepseek-cli-agent")
        .build()?)
}

async fn github_get(url: &str) -> Result<String> {
    let token = get_github_token()?;
    let client = create_client()?;
    let resp = client
        .get(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?;

    let status = resp.status();
    let body = resp.text().await?;

    if !status.is_success() {
        return Ok(format!("GitHub API error ({}): {}", status.as_u16(), body));
    }
    Ok(body)
}

async fn github_post(url: &str, body: &serde_json::Value) -> Result<String> {
    let token = get_github_token()?;
    let client = create_client()?;
    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .json(body)
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Ok(format!(
            "GitHub API error ({}): {}",
            status.as_u16(),
            body_text
        ));
    }
    Ok(body_text)
}

async fn github_patch(url: &str, body: &serde_json::Value) -> Result<String> {
    let token = get_github_token()?;
    let client = create_client()?;
    let resp = client
        .patch(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .json(body)
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Ok(format!(
            "GitHub API error ({}): {}",
            status.as_u16(),
            body_text
        ));
    }
    Ok(body_text)
}

// ─── Helper: parse owner/repo ───────────────────────────────────────

fn parse_repo(repo: &str) -> Result<(&str, &str)> {
    let parts: Vec<&str> = repo.split('/').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid repo format. Use 'owner/repo'.");
    }
    Ok((parts[0], parts[1]))
}

// ─── Repository Operations ──────────────────────────────────────────

pub async fn github_repo_info(repo: &str) -> Result<String> {
    let (owner, name) = parse_repo(repo)?;
    let url = format!("https://api.github.com/repos/{}/{}", owner, name);
    github_get(&url).await
}

pub async fn github_repo_list_issues(
    repo: &str,
    state: Option<&str>,
    limit: Option<usize>,
) -> Result<String> {
    let (owner, name) = parse_repo(repo)?;
    let s = state.unwrap_or("open");
    let per_page = limit.unwrap_or(10);
    let url = format!(
        "https://api.github.com/repos/{}/{}/issues?state={}&per_page={}",
        owner, name, s, per_page
    );
    let body = github_get(&url).await?;

    // Simplify the JSON output
    let issues: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap_or_default();
    let summary: Vec<String> = issues
        .iter()
        .map(|i| {
            format!(
                "#{} {} [{}] ({})",
                i["number"].as_u64().unwrap_or(0),
                i["title"].as_str().unwrap_or(""),
                i["state"].as_str().unwrap_or(""),
                i["html_url"].as_str().unwrap_or(""),
            )
        })
        .collect();
    Ok(summary.join("\n"))
}

// ─── Issue Operations ───────────────────────────────────────────────

pub async fn github_issue_create(
    repo: &str,
    title: &str,
    body: Option<&str>,
    labels: Option<&str>,
) -> Result<String> {
    let (owner, name) = parse_repo(repo)?;
    let url = format!("https://api.github.com/repos/{}/{}/issues", owner, name);

    let mut json = serde_json::json!({ "title": title });
    if let Some(b) = body {
        json["body"] = serde_json::Value::String(b.to_string());
    }
    if let Some(l) = labels {
        let label_vec: Vec<&str> = l.split(',').map(|s| s.trim()).collect();
        json["labels"] = serde_json::json!(label_vec);
    }

    let resp = github_post(&url, &json).await?;
    let issue: serde_json::Value = serde_json::from_str(&resp).unwrap_or_default();
    Ok(format!(
        "Issue #{} created: {}",
        issue["number"].as_u64().unwrap_or(0),
        issue["html_url"].as_str().unwrap_or(""),
    ))
}

pub async fn github_issue_update(
    repo: &str,
    issue_number: u64,
    title: Option<&str>,
    body: Option<&str>,
    state: Option<&str>,
) -> Result<String> {
    let (owner, name) = parse_repo(repo)?;
    let url = format!(
        "https://api.github.com/repos/{}/{}/issues/{}",
        owner, name, issue_number
    );

    let mut json = serde_json::json!({});
    if let Some(t) = title {
        json["title"] = serde_json::Value::String(t.to_string());
    }
    if let Some(b) = body {
        json["body"] = serde_json::Value::String(b.to_string());
    }
    if let Some(s) = state {
        json["state"] = serde_json::Value::String(s.to_string());
    }

    github_patch(&url, &json).await
}

// ─── Pull Request Operations ────────────────────────────────────────

pub async fn github_pr_list(
    repo: &str,
    state: Option<&str>,
    limit: Option<usize>,
) -> Result<String> {
    let (owner, name) = parse_repo(repo)?;
    let s = state.unwrap_or("open");
    let per_page = limit.unwrap_or(10);
    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls?state={}&per_page={}",
        owner, name, s, per_page
    );
    let body = github_get(&url).await?;

    let prs: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap_or_default();
    let summary: Vec<String> = prs
        .iter()
        .map(|pr| {
            format!(
                "#{} {} [{}] -> [{}] ({})",
                pr["number"].as_u64().unwrap_or(0),
                pr["title"].as_str().unwrap_or(""),
                pr["head"]["ref"].as_str().unwrap_or(""),
                pr["base"]["ref"].as_str().unwrap_or(""),
                pr["html_url"].as_str().unwrap_or(""),
            )
        })
        .collect();

    if summary.is_empty() {
        Ok("No pull requests found.".to_string())
    } else {
        Ok(summary.join("\n"))
    }
}

pub async fn github_pr_create(
    repo: &str,
    title: &str,
    head: &str,
    base: &str,
    body: Option<&str>,
    draft: bool,
) -> Result<String> {
    let (owner, name) = parse_repo(repo)?;
    let url = format!("https://api.github.com/repos/{}/{}/pulls", owner, name);

    let mut json = serde_json::json!({
        "title": title,
        "head": head,
        "base": base,
    });
    if let Some(b) = body {
        json["body"] = serde_json::Value::String(b.to_string());
    }
    if draft {
        json["draft"] = serde_json::Value::Bool(true);
    }

    let resp = github_post(&url, &json).await?;
    let pr: serde_json::Value = serde_json::from_str(&resp).unwrap_or_default();
    Ok(format!(
        "PR #{} created: {}",
        pr["number"].as_u64().unwrap_or(0),
        pr["html_url"].as_str().unwrap_or(""),
    ))
}

pub async fn github_pr_info(repo: &str, pr_number: u64) -> Result<String> {
    let (owner, name) = parse_repo(repo)?;
    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls/{}",
        owner, name, pr_number
    );
    github_get(&url).await
}

pub async fn github_pr_merge(repo: &str, pr_number: u64, method: Option<&str>) -> Result<String> {
    let (owner, name) = parse_repo(repo)?;
    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls/{}/merge",
        owner, name, pr_number
    );

    let merge_method = method.unwrap_or("merge");
    let json = serde_json::json!({ "merge_method": merge_method });

    let resp = github_post(&url, &json).await?;
    Ok(resp)
}

// ─── Search ─────────────────────────────────────────────────────────

pub async fn github_search_code(
    query: &str,
    repo: Option<&str>,
    limit: Option<usize>,
) -> Result<String> {
    let token = get_github_token()?;
    let client = create_client()?;
    let per_page = limit.unwrap_or(10);

    let q = if let Some(r) = repo {
        format!("{} repo:{}", query, r)
    } else {
        query.to_string()
    };

    let url = format!(
        "https://api.github.com/search/code?q={}&per_page={}",
        urlencoding(&q),
        per_page
    );

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?;

    let body = resp.text().await?;
    let search_result: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    let items = search_result["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let summary: Vec<String> = items
        .iter()
        .map(|item| {
            format!(
                "{} ({}) - {}",
                item["path"].as_str().unwrap_or(""),
                item["repository"]["full_name"].as_str().unwrap_or(""),
                item["html_url"].as_str().unwrap_or(""),
            )
        })
        .collect();

    let total = search_result["total_count"].as_u64().unwrap_or(0);
    Ok(format!("Found {} results:\n{}", total, summary.join("\n")))
}

pub async fn github_search_repos(query: &str, limit: Option<usize>) -> Result<String> {
    let token = get_github_token()?;
    let client = create_client()?;
    let per_page = limit.unwrap_or(10);

    let url = format!(
        "https://api.github.com/search/repositories?q={}&per_page={}",
        urlencoding(query),
        per_page
    );

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?;

    let body = resp.text().await?;
    let search_result: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    let items = search_result["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let summary: Vec<String> = items
        .iter()
        .map(|repo| {
            format!(
                "{} ⭐{} {} - {}",
                repo["full_name"].as_str().unwrap_or(""),
                repo["stargazers_count"].as_u64().unwrap_or(0),
                repo["language"].as_str().unwrap_or(""),
                repo["html_url"].as_str().unwrap_or(""),
            )
        })
        .collect();

    let total = search_result["total_count"].as_u64().unwrap_or(0);
    Ok(format!(
        "Found {} repositories:\n{}",
        total,
        summary.join("\n")
    ))
}

// ─── File Content ───────────────────────────────────────────────────

pub async fn github_get_file(repo: &str, path: &str, ref_: Option<&str>) -> Result<String> {
    let (owner, name) = parse_repo(repo)?;
    let r = ref_.unwrap_or("main");
    let url = format!(
        "https://api.github.com/repos/{}/{}/contents/{}?ref={}",
        owner, name, path, r
    );

    let body = github_get(&url).await?;
    let file_info: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();

    if let Some(content) = file_info["content"].as_str() {
        let decoded =
            base64_decode(content).unwrap_or_else(|| "Unable to decode content.".to_string());
        Ok(decoded)
    } else {
        Ok(body)
    }
}

// ─── Actions / Workflows ────────────────────────────────────────────

pub async fn github_workflow_list(repo: &str) -> Result<String> {
    let (owner, name) = parse_repo(repo)?;
    let url = format!(
        "https://api.github.com/repos/{}/{}/actions/workflows",
        owner, name
    );
    github_get(&url).await
}

pub async fn github_workflow_runs(
    repo: &str,
    workflow_id: Option<&str>,
    limit: Option<usize>,
) -> Result<String> {
    let (owner, name) = parse_repo(repo)?;
    let per_page = limit.unwrap_or(10);

    let url = if let Some(wf) = workflow_id {
        format!(
            "https://api.github.com/repos/{}/{}/actions/workflows/{}/runs?per_page={}",
            owner, name, wf, per_page
        )
    } else {
        format!(
            "https://api.github.com/repos/{}/{}/actions/runs?per_page={}",
            owner, name, per_page
        )
    };

    let body = github_get(&url).await?;
    let runs: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    let items = runs["workflow_runs"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let summary: Vec<String> = items
        .iter()
        .map(|run| {
            format!(
                "#{} {} [{}] {} - {}",
                run["run_number"].as_u64().unwrap_or(0),
                run["name"].as_str().unwrap_or(""),
                run["status"].as_str().unwrap_or(""),
                run["conclusion"].as_str().unwrap_or("pending"),
                run["html_url"].as_str().unwrap_or(""),
            )
        })
        .collect();

    if summary.is_empty() {
        Ok("No workflow runs found.".to_string())
    } else {
        Ok(summary.join("\n"))
    }
}

// ─── Utilities ──────────────────────────────────────────────────────

fn urlencoding(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char)
            }
            b' ' => result.push('+'),
            _ => result.push_str(&format!("%{:02X}", byte)),
        }
    }
    result
}

fn base64_decode(input: &str) -> Option<String> {
    use std::collections::HashMap;

    let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    let alphabet: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        .chars()
        .collect();
    let mut index_map = HashMap::new();
    for (i, c) in alphabet.iter().enumerate() {
        index_map.insert(*c, i as u8);
    }

    let len = cleaned.len();
    if len % 4 != 0 {
        return None;
    }

    let mut bytes = Vec::new();
    let chars: Vec<char> = cleaned.chars().collect();

    for chunk in chars.chunks(4) {
        let mut buf: [u8; 4] = [0; 4];
        for (i, c) in chunk.iter().enumerate() {
            if *c == '=' {
                buf[i] = 0;
            } else {
                buf[i] = *index_map.get(c)?;
            }
        }

        bytes.push((buf[0] << 2) | (buf[1] >> 4));
        if chunk.get(2) != Some(&'=') {
            bytes.push((buf[1] << 4) | (buf[2] >> 2));
        }
        if chunk.get(3) != Some(&'=') {
            bytes.push((buf[2] << 6) | buf[3]);
        }
    }

    String::from_utf8(bytes).ok()
}
