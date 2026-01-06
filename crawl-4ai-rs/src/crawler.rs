use chromiumoxide::browser::{Browser, BrowserConfig};
use futures::StreamExt;
use anyhow::{Result, anyhow};
use crate::models::CrawlResult;
use crate::markdown::DefaultMarkdownGenerator;
use std::env;
use std::path::Path;

#[derive(Default)]
pub struct AsyncWebCrawler {
    browser: Option<Browser>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl AsyncWebCrawler {
    pub fn new() -> Self {
        Self {
            browser: None,
            handle: None,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        if self.browser.is_some() {
            return Ok(());
        }

        let mut builder = BrowserConfig::builder();

        // Allow overriding via environment variable
        if let Ok(path) = env::var("CHROME_EXECUTABLE") {
            builder = builder.chrome_executable(Path::new(&path));
        } else {
             // Fallback for this specific environment if auto-detect fails
             // In a real library, we might want to rely on chromiumoxide's auto-detect,
             // but for this sandbox, we know where it is.
             // We'll leave it to auto-detect if env var is not set,
             // but if auto-detect fails in this env, users should set the env var.
             // However, to pass the test in this env without setting env var explicitly in the test code (which modifies global state),
             // we can check if the known path exists and use it if so.
             let known_path = Path::new("/usr/bin/google-chrome-stable");
             if known_path.exists() {
                 builder = builder.chrome_executable(known_path);
             }
        }

        let config = builder
            .arg("--no-sandbox")
            .arg("--disable-dev-shm-usage")
            .build()
            .map_err(|e| anyhow!(e))?;

        let (browser, mut handler) = Browser::launch(config).await?;

        let handle = tokio::task::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        self.browser = Some(browser);
        self.handle = Some(handle);

        Ok(())
    }

    pub async fn arun(&mut self, url: &str) -> Result<CrawlResult> {
        if self.browser.is_none() {
            self.start().await?;
        }

        let browser = self.browser.as_ref().unwrap();
        let page = browser.new_page(url).await?;

        page.wait_for_navigation().await?;
        let html = page.content().await?;

        page.close().await?;

        // Generate Markdown
        let generator = DefaultMarkdownGenerator::new();
        let markdown_result = generator.generate_markdown(&html);

        Ok(CrawlResult {
            url: url.to_string(),
            html,
            success: true,
            cleaned_html: None,
            media: None,
            links: None,
            screenshot: None,
            markdown: Some(markdown_result),
            extracted_content: None,
            error_message: None,
        })
    }
}
