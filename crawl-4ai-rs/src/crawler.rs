use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::target::{CreateBrowserContextParams, CreateTargetParams};
use chromiumoxide::cdp::browser_protocol::browser::BrowserContextId;
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::page::ScreenshotParams;
use futures::StreamExt;
use anyhow::{Result, anyhow};
use crate::models::{CrawlResult, MediaItem, Link, CrawlerRunConfig, WaitStrategy};
use crate::markdown::DefaultMarkdownGenerator;
use crate::content_filter::{PruningContentFilter, ContentFilter};
use std::env;
use std::path::Path;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::Deserialize;
use thiserror::Error;

/// Errors that can occur during the crawling process.
#[derive(Error, Debug)]
pub enum CrawlerError {
    /// Error related to the browser instance or connection.
    #[error("Browser error: {0}")]
    BrowserError(String),
    /// Error occurring during page navigation.
    #[error("Navigation error: {0}")]
    NavigationError(String),
    /// Timeout error when waiting for a condition (selector, JS).
    #[error("Timeout waiting for {0}")]
    Timeout(String),
    /// Error during content extraction.
    #[error("Extraction error: {0}")]
    ExtractionError(String),
    /// Other miscellaneous errors.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// An asynchronous web crawler based on `chromiumoxide`.
///
/// This struct manages the browser instance, sessions, and the crawling process.
/// It supports features like:
/// - Headless crawling
/// - Session management (persistent contexts)
/// - Markdown generation
/// - Content filtering (Pruning, BM25, LLM)
/// - Screenshot capture
/// - JavaScript execution for extraction
#[derive(Default)]
pub struct AsyncWebCrawler {
    browser: Option<Browser>,
    handle: Option<tokio::task::JoinHandle<()>>,
    sessions: HashMap<String, BrowserContextId>,
}

#[derive(Deserialize)]
struct ExtractionResult {
    media: HashMap<String, Vec<MediaItem>>,
    links: HashMap<String, Vec<Link>>,
}

impl AsyncWebCrawler {
    /// Creates a new instance of `AsyncWebCrawler`.
    pub fn new() -> Self {
        Self {
            browser: None,
            handle: None,
            sessions: HashMap::new(),
        }
    }

    /// Starts the browser instance.
    ///
    /// This method launches a headless Chromium instance. It attempts to locate the
    /// Chrome executable automatically or uses the `CHROME_EXECUTABLE` environment variable.
    /// It also spawns a background task to handle browser events.
    pub async fn start(&mut self) -> Result<()> {
        if self.browser.is_some() {
            // Check if the handler is still running
            if let Some(h) = &self.handle {
                if !h.is_finished() {
                    return Ok(());
                }
            }
            // If finished, we need to restart, so clean up
            self.browser = None;
            self.handle = None;
            // Also clean sessions as the browser context is gone
            self.sessions.clear();
        }

        let mut builder = BrowserConfig::builder();

        // Allow overriding via environment variable
        if let Ok(path) = env::var("CHROME_EXECUTABLE") {
            builder = builder.chrome_executable(Path::new(&path));
        } else {
             // Fallback: check for chromium as well since it's common in linux envs
             let known_paths = [
                 "/usr/bin/google-chrome-stable",
                 "/usr/bin/chromium",
                 "/usr/bin/chromium-browser"
             ];

             for path_str in known_paths {
                 let path = Path::new(path_str);
                 if path.exists() {
                     builder = builder.chrome_executable(path);
                     break;
                 }
             }
        }

        let config = builder
            .arg("--no-sandbox")
            .arg("--disable-dev-shm-usage")
            .arg("--disable-gpu")
            .arg("--disable-setuid-sandbox")
            .build()
            .map_err(|e| anyhow!(e))?;

        let (browser, mut handler) = Browser::launch(config).await?;

        let handle = tokio::task::spawn(async move {
            while let Some(h) = handler.next().await {
                if let Err(e) = h {
                    eprintln!("Browser handler error: {:?}", e);
                    continue;
                }
            }
            eprintln!("Browser handler loop exited");
        });

        self.browser = Some(browser);
        self.handle = Some(handle);

        Ok(())
    }

    /// Asynchronously crawls a URL with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to crawl.
    /// * `config` - Optional configuration for the crawl run (wait strategies, content filters, etc.).
    ///
    /// # Returns
    ///
    /// A `Result` containing `CrawlResult` on success, or an error.
    ///
    /// # Retry Logic
    ///
    /// This method includes a retry mechanism (up to 3 attempts) for handling transient
    /// errors like browser startup failures or session creation issues.
    pub async fn arun(&mut self, url: &str, config: Option<CrawlerRunConfig>) -> Result<CrawlResult> {
        let max_retries = 3;
        let mut attempt = 0;

        loop {
            attempt += 1;

            // 1. Ensure browser is running
            if self.browser.is_none() || self.handle.as_ref().map(|h| h.is_finished()).unwrap_or(true) {
                if let Err(e) = self.start().await {
                    if attempt >= max_retries {
                         return Err(CrawlerError::BrowserError(format!("Failed to start browser: {}", e)).into());
                    }
                    eprintln!("Failed to start browser (attempt {}): {}", attempt, e);
                    tokio::time::sleep(Duration::from_millis(500 * attempt)).await;
                    continue;
                }
            }

            // 2. Clone browser handle and manage session
            let browser = self.browser.as_ref().unwrap();

            // Handle session creation here (using &mut self)
            let context_id = if let Some(ref cfg) = config {
                if let Some(ref session_id) = cfg.session_id {
                     // Check if session exists
                     if let Some(id) = self.sessions.get(session_id) {
                         Some(id.clone())
                     } else {
                         // Create new session
                         // Note: We use the cloned browser handle, so no conflict with &mut self
                         match browser.create_browser_context(CreateBrowserContextParams::default()).await {
                             Ok(id) => {
                                 self.sessions.insert(session_id.clone(), id.clone());
                                 Some(id)
                             },
                             Err(e) => {
                                 // If session creation fails, it's a browser error
                                 let err_str = e.to_string();
                                 if attempt >= max_retries {
                                     return Err(CrawlerError::BrowserError(format!("Failed to create session: {}", e)).into());
                                 }
                                 eprintln!("Failed to create session (attempt {}): {}", attempt, e);
                                 // If connection failed, invalidate browser
                                 if err_str.contains("oneshot canceled") || err_str.contains("channel closed") {
                                     self.browser = None;
                                 }
                                 tokio::time::sleep(Duration::from_millis(500 * attempt)).await;
                                 continue;
                             }
                         }
                     }
                } else {
                    None
                }
            } else {
                None
            };

            // 3. Perform the crawl logic
            // From this point on, we don't need &mut self anymore, we use `browser` and `context_id`.
            // However, we are inside a loop that requires &mut self for the next iteration (start()).
            // So we can call an async block or function.

            let result: Result<CrawlResult> = async {
                let page = if let Some(cid) = context_id {
                    let params = CreateTargetParams::builder()
                        .url(url)
                        .browser_context_id(cid)
                        .build()
                        .map_err(|e| anyhow!(e))?;
                    browser.new_page(params).await?
                } else {
                    browser.new_page(url).await?
                };

                page.wait_for_navigation().await?;

                if let Some(ref cfg) = config {
                    if let Some(ref strategy) = cfg.wait_for {
                        match strategy {
                            WaitStrategy::Fixed(ms) => {
                                tokio::time::sleep(Duration::from_millis(*ms)).await;
                            },
                            WaitStrategy::Selector(selector) => {
                                let timeout = Duration::from_secs(10);
                                let start = Instant::now();
                                loop {
                                    if start.elapsed() > timeout {
                                        eprintln!("Timeout waiting for selector: {}", selector);
                                        break;
                                    }
                                    match page.find_element(selector).await {
                                        Ok(_) => break,
                                        Err(_) => {
                                            tokio::time::sleep(Duration::from_millis(500)).await;
                                        }
                                    }
                                }
                            },
                            WaitStrategy::JsCondition(js) => {
                                 let timeout = Duration::from_secs(10);
                                 let start = Instant::now();
                                 loop {
                                    if start.elapsed() > timeout {
                                        eprintln!("Timeout waiting for js condition");
                                        break;
                                    }
                                    match page.evaluate(js.as_str()).await {
                                        Ok(val) => {
                                            if let Ok(bool_val) = val.into_value::<bool>() {
                                                if bool_val {
                                                    break;
                                                }
                                            }
                                        },
                                        Err(_) => {}
                                    }
                                    tokio::time::sleep(Duration::from_millis(500)).await;
                                 }
                            }
                        }
                    }
                }

                let html = page.content().await?;

                let screenshot_data = if let Some(ref cfg) = config {
                    if cfg.screenshot {
                        let params = ScreenshotParams::builder()
                            .format(CaptureScreenshotFormat::Png)
                            .full_page(true)
                            .build();

                        match page.screenshot(params).await {
                            Ok(bytes) => {
                                use base64::{Engine as _, engine::general_purpose};
                                Some(general_purpose::STANDARD.encode(bytes))
                            },
                            Err(e) => {
                                eprintln!("Failed to take screenshot: {}", e);
                                None
                            }
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Extract media and links using JavaScript
                let script = r#"
                    (() => {
                        const resolveUrl = (url) => {
                            try {
                                return new URL(url, document.baseURI).href;
                            } catch (e) {
                                return url;
                            }
                        };

                        const media = {};
                        const images = Array.from(document.images).map(img => ({
                            src: resolveUrl(img.src),
                            alt: img.alt || null,
                            desc: img.title || null,
                            score: null,
                            type: "image",
                            group_id: null
                        }));
                        media["images"] = images;

                        const links = { internal: [], external: [] };
                        const domain = window.location.hostname;

                        Array.from(document.links).forEach(link => {
                            const href = resolveUrl(link.href);
                            const linkObj = {
                                href: href,
                                text: link.innerText || null,
                                title: link.title || null
                            };

                            try {
                                const linkUrl = new URL(href);
                                if (linkUrl.hostname && linkUrl.hostname === domain) {
                                    links.internal.push(linkObj);
                                } else {
                                    links.external.push(linkObj);
                                }
                            } catch (e) {
                                links.external.push(linkObj);
                            }
                        });

                        return { media, links };
                    })()
                "#;

                let extraction: Option<ExtractionResult> = match page.evaluate(script).await {
                    Ok(val) => match val.into_value() {
                        Ok(v) => Some(v),
                        Err(e) => {
                            eprintln!("Failed to deserialize extraction result: {:?}", e);
                            None
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to evaluate extraction script: {:?}", e);
                        None
                    }
                };

                page.close().await?;

                // Generate Markdown
                let content_filter = if let Some(ref cfg) = config {
                    cfg.content_filter.clone().unwrap_or(ContentFilter::Pruning(PruningContentFilter::default()))
                } else {
                    ContentFilter::Pruning(PruningContentFilter::default())
                };

                let generator = DefaultMarkdownGenerator::new(Some(content_filter));
                let markdown_result = generator.generate_markdown(&html).await;

                let (media, links) = if let Some(ext) = extraction {
                    (Some(ext.media), Some(ext.links))
                } else {
                    (None, None)
                };

                Ok(CrawlResult {
                    url: url.to_string(),
                    html,
                    success: true,
                    cleaned_html: None,
                    media,
                    links,
                    screenshot: screenshot_data,
                    markdown: Some(markdown_result),
                    extracted_content: None,
                    error_message: None,
                })
            }.await;

            match result {
                Ok(res) => return Ok(res),
                Err(e) => {
                     let err_str = e.to_string();
                     // Check if it's a fatal browser error
                     let is_fatal = err_str.contains("oneshot canceled") || err_str.contains("channel closed") || err_str.contains("Broken pipe") || err_str.contains("Connection reset by peer");

                     if is_fatal || attempt < max_retries {
                         eprintln!("Crawl error (attempt {}/{}): {}", attempt, max_retries, err_str);
                         if is_fatal {
                             self.browser = None;
                             // We should also probably clear sessions, as context IDs are invalid
                             self.sessions.clear();
                         }
                         if attempt >= max_retries {
                             return Err(e);
                         }
                         tokio::time::sleep(Duration::from_millis(500 * attempt)).await;
                         continue;
                     }
                     return Err(e);
                }
            }
        }
    }
}
