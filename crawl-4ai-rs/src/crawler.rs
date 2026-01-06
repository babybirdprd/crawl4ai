use chromiumoxide::browser::{Browser, BrowserConfig};
use futures::StreamExt;
use anyhow::{Result, anyhow};
use crate::models::{CrawlResult, MediaItem, Link};
use crate::markdown::DefaultMarkdownGenerator;
use std::env;
use std::collections::HashMap;
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

        // Extract Media
        let media_items: Vec<MediaItem> = page.evaluate(r#"
            Array.from(document.querySelectorAll('img')).map((img) => ({
                src: img.src,
                alt: img.alt,
                desc: img.title || '',
                score: 0,
                type_: 'image',
                group_id: 0
            }))
        "#).await?.into_value()?;

        let mut media = HashMap::new();
        media.insert("images".to_string(), media_items);

        // Extract Links
        let links_list: Vec<Link> = page.evaluate(r#"
            Array.from(document.querySelectorAll('a')).map(a => ({
                href: a.href,
                text: a.innerText,
                title: a.title
            }))
        "#).await?.into_value()?;

        let mut links = HashMap::new();
        links.insert("internal".to_string(), links_list); // Categorize later properly

        page.close().await?;

        // Generate Markdown
        let generator = DefaultMarkdownGenerator::new();
        let markdown_result = generator.generate_markdown(&html);

        Ok(CrawlResult {
            url: url.to_string(),
            html,
            success: true,
            cleaned_html: None,
            media: Some(media),
            links: Some(links),
            screenshot: None,
            markdown: Some(markdown_result),
            extracted_content: None,
            error_message: None,
        })
    }
}
