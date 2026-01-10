use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::target::{CreateBrowserContextParams, CreateTargetParams};
use chromiumoxide::cdp::browser_protocol::network::{self, EventRequestWillBeSent, EventLoadingFinished, EventLoadingFailed};
use chromiumoxide::cdp::browser_protocol::browser::BrowserContextId;
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::page::ScreenshotParams;
use futures::StreamExt;
use anyhow::{Result, anyhow};
use crate::models::{CrawlResult, MediaItem, Link, CrawlerRunConfig, WaitStrategy, ExtractionStrategyConfig};
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
    pub async fn start(&mut self) -> Result<()> {
        if self.browser.is_some() {
            if let Some(h) = &self.handle {
                if !h.is_finished() {
                    return Ok(());
                }
            }
            self.browser = None;
            self.handle = None;
            self.sessions.clear();
        }

        let mut builder = BrowserConfig::builder();

        if let Ok(path) = env::var("CHROME_EXECUTABLE") {
            builder = builder.chrome_executable(Path::new(&path));
        } else {
             let known_paths = [
                 "/usr/bin/google-chrome-stable",
                 "/usr/bin/chromium",
                 "/usr/bin/chromium-browser",
                 "/home/jules/.cache/ms-playwright/chromium-1187/chrome-linux/chrome"
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
    pub async fn arun(&mut self, url: &str, config: Option<CrawlerRunConfig>) -> Result<CrawlResult> {
        let max_retries = 3;
        let base_delay = 500;

        let mut attempt = 0;

        loop {
            attempt += 1;

            // 1. Ensure browser is ready
            if let Err(e) = self.ensure_browser_ready(attempt).await {
                if attempt >= max_retries { return Err(e); }
                tokio::time::sleep(Duration::from_millis(base_delay * attempt as u64)).await;
                continue;
            }

            // 2. Prepare session
            // We use a scope here to limit the lifetime of the browser borrow if possible
            let context_id_result = {
                let browser = self.browser.as_ref().unwrap();
                Self::prepare_session(browser, &mut self.sessions, &config).await
            };

            let context_id = match context_id_result {
                Ok(id) => id,
                Err(e) => {
                     let err_str = e.to_string();
                     if Self::is_fatal_error(&err_str) {
                         self.reset_browser();
                     }
                     if attempt >= max_retries {
                         return Err(e);
                     }
                     eprintln!("Session preparation failed (attempt {}): {}", attempt, e);
                     tokio::time::sleep(Duration::from_millis(base_delay * attempt as u64)).await;
                     continue;
                }
            };

            // 3. Execute Crawl
            // We need to re-borrow browser here. This is fine because the previous borrow ended after the block?
            // Wait, context_id_result borrow? No, context_id is Option<BrowserContextId> which is just a String wrapper usually.
            // Let's check if BrowserContextId borrows anything. It shouldn't.

            let browser = self.browser.as_ref().unwrap();
            let crawl_result = Self::crawl_page(browser, context_id, url, &config).await;

            match crawl_result {
                Ok(res) => return Ok(res),
                Err(e) => {
                    let err_str = e.to_string();
                    let is_fatal = Self::is_fatal_error(&err_str);

                    if is_fatal || attempt < max_retries {
                         eprintln!("Crawl error (attempt {}/{}): {}", attempt, max_retries, err_str);
                         if is_fatal {
                             self.reset_browser();
                         }
                         if attempt >= max_retries {
                             return Err(e);
                         }
                         tokio::time::sleep(Duration::from_millis(base_delay * attempt as u64)).await;
                         continue;
                    }
                    return Err(e);
                }
            }
        }
    }

    async fn ensure_browser_ready(&mut self, attempt: u32) -> Result<()> {
        if self.browser.is_none() || self.handle.as_ref().map(|h| h.is_finished()).unwrap_or(true) {
            if let Err(e) = self.start().await {
                eprintln!("Failed to start browser (attempt {}): {}", attempt, e);
                return Err(CrawlerError::BrowserError(format!("Failed to start browser: {}", e)).into());
            }
        }
        Ok(())
    }

    fn reset_browser(&mut self) {
        self.browser = None;
        self.sessions.clear();
    }

    fn is_fatal_error(err_str: &str) -> bool {
        err_str.contains("oneshot canceled") ||
        err_str.contains("channel closed") ||
        err_str.contains("Broken pipe") ||
        err_str.contains("Connection reset by peer")
    }

    async fn prepare_session(
        browser: &Browser,
        sessions: &mut HashMap<String, BrowserContextId>,
        config: &Option<CrawlerRunConfig>
    ) -> Result<Option<BrowserContextId>> {
        if let Some(ref cfg) = config {
            if let Some(ref session_id) = cfg.session_id {
                 if let Some(id) = sessions.get(session_id) {
                     return Ok(Some(id.clone()));
                 } else {
                     let id = browser.create_browser_context(CreateBrowserContextParams::default()).await
                        .map_err(|e| CrawlerError::BrowserError(format!("Failed to create session: {}", e)))?;
                     sessions.insert(session_id.clone(), id.clone());
                     return Ok(Some(id));
                 }
            }
        }
        Ok(None)
    }

    /// Internal method to perform the actual page visit and extraction.
    async fn crawl_page(
        browser: &Browser,
        context_id: Option<BrowserContextId>,
        url: &str,
        config: &Option<CrawlerRunConfig>
    ) -> Result<CrawlResult> {
        let page = if let Some(cid) = context_id {
            let params = CreateTargetParams::builder()
                .url("about:blank")
                .browser_context_id(cid)
                .build()
                .map_err(|e| anyhow!(e))?;
            browser.new_page(params).await?
        } else {
            browser.new_page("about:blank").await?
        };

        let navigation_task = page.goto(url);
        if let Some(timeout_ms) = config.as_ref().and_then(|c| c.page_timeout) {
             match tokio::time::timeout(Duration::from_millis(timeout_ms), navigation_task).await {
                 Ok(res) => { res?; },
                 Err(_) => return Err(CrawlerError::Timeout("Page navigation timed out".to_string()).into()),
             }
        } else {
             navigation_task.await?;
        }

        if let Some(ref cfg) = config {
            if let Some(ref strategy) = cfg.wait_for {
                let timeout_ms = cfg.wait_timeout.unwrap_or(10_000);
                let timeout = Duration::from_millis(timeout_ms);
                let start = Instant::now();

                match strategy {
                    WaitStrategy::Fixed(ms) => {
                        tokio::time::sleep(Duration::from_millis(*ms)).await;
                    },
                    WaitStrategy::Selector(selector) => {
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
                    WaitStrategy::XPath(xpath) => {
                         // Escape backslashes first, then quotes to prevent injection issues
                         let escaped_xpath = xpath.replace("\\", "\\\\").replace("\"", "\\\"");
                         let js = format!(
                             r#"
                             (() => {{
                                 const result = document.evaluate("{}", document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
                                 return result.singleNodeValue !== null;
                             }})()
                             "#,
                             escaped_xpath
                         );
                         loop {
                            if start.elapsed() > timeout {
                                eprintln!("Timeout waiting for xpath: {}", xpath);
                                break;
                            }
                            match page.evaluate(js.as_str()).await {
                                Ok(val) => {
                                    if let Ok(true) = val.into_value::<bool>() {
                                        break;
                                    }
                                },
                                Err(_) => {}
                            }
                            tokio::time::sleep(Duration::from_millis(500)).await;
                        }
                    },
                    WaitStrategy::JsCondition(js) => {
                         loop {
                            if start.elapsed() > timeout {
                                eprintln!("Timeout waiting for js condition");
                                break;
                            }
                            match page.evaluate(js.as_str()).await {
                                Ok(val) => {
                                    if let Ok(true) = val.into_value::<bool>() {
                                        break;
                                    }
                                },
                                Err(_) => {}
                            }
                            tokio::time::sleep(Duration::from_millis(500)).await;
                         }
                    },
                    WaitStrategy::NetworkIdle { idle_time } => {
                        if let Err(e) = page.execute(network::EnableParams::default()).await {
                             eprintln!("Failed to enable network domain for idle wait: {}", e);
                        } else {
                            // Note: This implementation only tracks requests initiated AFTER this point.
                            // In-flight requests started before this block are not counted.
                            let request_sent = page.event_listener::<EventRequestWillBeSent>().await;
                            let request_finished = page.event_listener::<EventLoadingFinished>().await;
                            let request_failed = page.event_listener::<EventLoadingFailed>().await;

                            if let (Ok(mut request_sent), Ok(mut request_finished), Ok(mut request_failed)) =
                                (request_sent, request_finished, request_failed)
                            {
                                let mut active_requests = 0;
                                let mut last_activity = Instant::now();
                                let required_idle_time = Duration::from_millis(idle_time.unwrap_or(500));
                                let start_wait = Instant::now();

                                loop {
                                    if start_wait.elapsed() > timeout {
                                        eprintln!("Timeout waiting for network idle");
                                        break;
                                    }

                                    if active_requests == 0 && last_activity.elapsed() > required_idle_time {
                                        break;
                                    }

                                    tokio::select! {
                                        _ = tokio::time::sleep(Duration::from_millis(100)) => {
                                            // Periodic check
                                        }
                                        Some(_) = request_sent.next() => {
                                            active_requests += 1;
                                            last_activity = Instant::now();
                                        }
                                        Some(_) = request_finished.next() => {
                                            if active_requests > 0 { active_requests -= 1; }
                                            last_activity = Instant::now();
                                        }
                                        Some(_) = request_failed.next() => {
                                            if active_requests > 0 { active_requests -= 1; }
                                            last_activity = Instant::now();
                                        }
                                    }
                                }
                            } else {
                                eprintln!("Failed to attach event listeners for network idle");
                            }
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

        // Execute extraction strategy if present
        let extracted_content = if let Some(ref cfg) = config {
            if let Some(ref strategy) = cfg.extraction_strategy {
                 let results = match strategy {
                     ExtractionStrategyConfig::JsonCss(s) => s.extract(&html),
                     ExtractionStrategyConfig::JsonXPath(s) => s.extract(&html),
                     ExtractionStrategyConfig::Regex(s) => s.extract(url, &html),
                 };
                 match serde_json::to_string(&results) {
                     Ok(s) => Some(s),
                     Err(e) => {
                         eprintln!("Failed to serialize extraction results: {}", e);
                         None
                     }
                 }
            } else {
                None
            }
        } else {
            None
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
            extracted_content,
            error_message: None,
        })
    }
}

/// Generic retry logic with exponential backoff.
#[allow(dead_code)]
pub async fn retry_with_backoff<F, Fut, T, E>(
    mut operation: F,
    max_retries: u32,
    base_delay: u64,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display + std::fmt::Debug,
    anyhow::Error: From<E>,
{
    let mut attempt = 0;
    loop {
        attempt += 1;
        match operation().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                if attempt > max_retries {
                    return Err(anyhow::Error::from(e));
                }

                eprintln!("Operation failed (attempt {}/{}): {}", attempt, max_retries, e);
                tokio::time::sleep(Duration::from_millis(base_delay * attempt as u64)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn test_retry_success() {
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();

        let result = retry_with_backoff(
            || async {
                let mut c = counter_clone.lock().unwrap();
                *c += 1;
                if *c < 2 {
                    Err(anyhow!("Fail"))
                } else {
                    Ok("Success")
                }
            },
            3,
            10,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(*counter.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn test_retry_failure() {
         let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();

        let result: Result<&str> = retry_with_backoff(
            || async {
                let mut c = counter_clone.lock().unwrap();
                *c += 1;
                Err(anyhow!("Fail forever"))
            },
            2,
            10,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(*counter.lock().unwrap(), 3); // 1 initial + 2 retries
    }
}
