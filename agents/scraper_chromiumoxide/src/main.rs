use anyhow::Result;
use serde::{Deserialize, Serialize};
use futures_util::StreamExt;

#[derive(Debug, Deserialize)]
struct Input {
    url: String,
    #[serde(default = "default_headless")] headless: bool,
}
fn default_headless() -> bool { true }

#[derive(Debug, Serialize)]
struct Output {
    status: String,
    url: String,
    title: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut buf = String::new();
    use tokio::io::{AsyncReadExt, stdin};
    stdin().read_to_string(&mut buf).await?;
    let inp: Input = serde_json::from_str(&buf)?;

    let cfg = chromiumoxide::BrowserConfig::builder()
        .no_sandbox()
        // headless defaults are acceptable; we can tweak later if needed
        .build()
        .map_err(anyhow::Error::msg)?;
    let (browser, mut handler) = chromiumoxide::Browser::launch(cfg).await?;

    // drive the handler in background
    tokio::spawn(async move { while let Some(_) = handler.next().await {} });

    let page = browser.new_page(inp.url.as_str()).await?;
    let _ = page.wait_for_navigation().await; // ignore error
    let title = page.get_title().await.ok().flatten();
    let out = Output { status: "ok".into(), url: inp.url, title };
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}
