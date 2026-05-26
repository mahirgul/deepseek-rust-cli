use anyhow::Result;
use futures::StreamExt;
use once_cell::sync::Lazy;

static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create reqwest client")
});

pub async fn fetch_url(url: &str) -> Result<String> {
    let response = CLIENT.get(url).send().await?;

    // Limit to 1MB
    let max_size = 1024 * 1024;
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk_res) = stream.next().await {
        let chunk = chunk_res?;
        if body.len() + chunk.len() > max_size {
            anyhow::bail!("Fetched content exceeds 1MB limit.");
        }
        body.extend_from_slice(&chunk);
    }

    let clean = html2text::from_read(&body[..], 80)?;
    Ok(clean)
}

pub fn get_env_var(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| "Not set".to_string())
}

pub async fn web_search_duckduckgo(query: &str) -> Result<String> {
    let response = CLIENT.get("https://html.duckduckgo.com/html/")
        .query(&[("q", query)])
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .send()
        .await?
        .error_for_status()?;

    let body = response.bytes().await?;
    let clean = html2text::from_read(&body[..], 80)?;

    let lines: Vec<&str> = clean
        .lines()
        .filter(|line| {
            let l = line.trim();
            !l.is_empty()
                && !l.contains("duckduckgo.com")
                && !l.starts_with("Ad")
                && !l.contains("JavaScript")
        })
        .collect();

    let count = lines.len().min(100);
    let result_text = lines[..count].join("\n");
    Ok(result_text)
}
