use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::target::{CreateBrowserContextParams, CreateTargetParams};
use chromiumoxide::cdp::browser_protocol::browser::BrowserContextId;
use chromiumoxide::cdp::browser_protocol::page::CaptureSnapshotFormat;
use chromiumoxide::cdp::browser_protocol::page::CaptureSnapshotParams;
use chromiumoxide::cdp::browser_protocol::network::{EventRequestWillBeSent, EventResponseReceived, EventLoadingFailed};
use chromiumoxide::cdp::js_protocol::runtime::EventConsoleApiCalled;
use futures::StreamExt;
use anyhow::{Result, anyhow};
use crate::models::{CrawlResult, MediaItem, Link, CrawlerRunConfig, WaitStrategy, NetworkRequest, ConsoleMessage, ContentSource};
use std::sync::{Arc, Mutex};
use crate::markdown::DefaultMarkdownGenerator;
use crate::content_filter::{PruningContentFilter, ContentFilter};
use std::env;
use std::path::Path;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CrawlerError {
    #[error("Browser error: {0}")]
    BrowserError(String),
    #[error("Navigation error: {0}")]
    NavigationError(String),
    #[error("Timeout waiting for {0}")]
    Timeout(String),
    #[error("Extraction error: {0}")]
    ExtractionError(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

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
    pub fn new() -> Self {
        Self {
            browser: None,
            handle: None,
            sessions: HashMap::new(),
        }
    }

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
                    // Create page first, but don't navigate yet?
                    // chromiumoxide::Browser::new_page navigates immediately.
                    // We need to set listeners BEFORE navigation to capture initial requests.
                    // But `new_page` returns a page that is already navigating or navigated?
                    // Docs say: "Triggers a navigation to the search result page" in example.
                    // Actually `new_page(url)` calls `CreateTarget` with url.

                    // If we want to capture everything, we should create a blank page, setup listeners, then navigate.

                    // However, `new_page` is convenient.
                    // Let's try to create a blank page first if we need to capture.

                    if config.as_ref().map(|c| c.capture_network_requests.unwrap_or(false) || c.capture_console_messages.unwrap_or(false)).unwrap_or(false) {
                         let page = browser.new_page("about:blank").await?;
                         // Return page here? No, we need to assign it to `page` variable but we are in if/else.
                         page
                    } else {
                         browser.new_page(url).await?
                    }
                };

                // Note: If we created about:blank, we need to navigate later.

                // Setup listeners for network and console capture
                let capture_network = config.as_ref().map(|c| c.capture_network_requests.unwrap_or(false)).unwrap_or(false);
                let capture_console = config.as_ref().map(|c| c.capture_console_messages.unwrap_or(false)).unwrap_or(false);

                let network_requests: Arc<Mutex<Vec<NetworkRequest>>> = Arc::new(Mutex::new(Vec::new()));
                let console_messages: Arc<Mutex<Vec<ConsoleMessage>>> = Arc::new(Mutex::new(Vec::new()));

                let _network_listener_handle;
                let _console_listener_handle;

                if capture_network {
                    // Enable Network domain
                    if let Err(e) = page.execute(chromiumoxide::cdp::browser_protocol::network::EnableParams::default()).await {
                        eprintln!("Failed to enable network: {:?}", e);
                    }

                    let requests = network_requests.clone();
                    let mut request_events = page.event_listener::<EventRequestWillBeSent>().await?;

                    _network_listener_handle = Some(tokio::spawn(async move {
                         while let Some(event) = request_events.next().await {
                             // eprintln!("Network event: {:?}", event.request.url);
                             let mut reqs = requests.lock().unwrap();
                             reqs.push(NetworkRequest {
                                 url: event.request.url.clone(),
                                 method: event.request.method.clone(),
                                 headers: Some(serde_json::from_value::<HashMap<String, String>>(serde_json::to_value(event.request.headers.clone()).unwrap()).unwrap_or_default()),
                                 response_status: None, // Filled later if we could match response
                                 response_headers: None,
                                 request_body: event.request.post_data.clone(),
                                 response_body: None,
                             });
                         }
                    }));
                } else {
                    _network_listener_handle = None;
                }

                if capture_console {
                     // Enable Runtime domain for console
                     if let Err(e) = page.enable_runtime().await {
                         eprintln!("Failed to enable runtime: {:?}", e);
                     }

                     let messages = console_messages.clone();
                     let mut console_events = page.event_listener::<EventConsoleApiCalled>().await?;
                     _console_listener_handle = Some(tokio::spawn(async move {
                         while let Some(event) = console_events.next().await {
                             let mut msgs = messages.lock().unwrap();
                             let text = event.args.iter()
                                .map(|arg| arg.value.as_ref().map(|v| v.to_string()).unwrap_or_default())
                                .collect::<Vec<_>>()
                                .join(" ");

                             msgs.push(ConsoleMessage {
                                 type_: format!("{:?}", event.r#type),
                                 text,
                                 source: None,
                                 line: None,
                                 column: None,
                                 url: None,
                             });
                         }
                     }));
                } else {
                    _console_listener_handle = None;
                }

                // If we started with about:blank (implied by capture flags), we need to navigate now.
                // Or if we just did new_page(url), we are already navigating.
                // But wait, if we did new_page(url), the navigation might have already happened or started.
                // Capturing after new_page(url) will miss initial requests.

                // So the logic above:
                // 1. If capture enabled -> new_page("about:blank") -> setup listeners -> goto(url)
                // 2. If capture disabled -> new_page(url) -> wait_for_navigation

                if capture_network || capture_console {
                    page.goto(url).await?;
                }

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

                let mhtml = if let Some(ref cfg) = config {
                    if cfg.capture_mhtml.unwrap_or(false) {
                        let params = CaptureSnapshotParams::builder()
                            .format(CaptureSnapshotFormat::Mhtml)
                            .build();
                        match page.execute(params).await {
                            Ok(res) => Some(res.data.clone()),
                            Err(e) => {
                                eprintln!("Failed to capture MHTML: {:?}", e);
                                None
                            }
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                page.close().await?;

                // Generate Markdown
                let content_filter = if let Some(ref cfg) = config {
                    cfg.content_filter.clone().unwrap_or(ContentFilter::Pruning(PruningContentFilter::default()))
                } else {
                    ContentFilter::Pruning(PruningContentFilter::default())
                };

                let generator = DefaultMarkdownGenerator::new(Some(content_filter.clone()));

                // Determine which HTML to use for markdown generation
                let source_html = if let Some(ref cfg) = config {
                    match cfg.content_source {
                        Some(ContentSource::RawHtml) => &html,
                        Some(ContentSource::CleanedHtml) | None => {
                            // Logic for cleaned HTML is handled by the generator filtering,
                            // but if we want to explicitly use the "filtered" output as input,
                            // we might need to filter first.
                            // Currently DefaultMarkdownGenerator takes the full HTML and filters internally using content_filter.
                            // So passing &html is correct for "CleanedHtml" strategy if generator does the cleaning.
                            &html
                        },
                    }
                } else {
                    &html
                };

                // Note: content_filter is already passed to generator.
                // If content_source is RawHtml, we might want to DISABLE content filtering in generator?
                // Or does RawHtml just mean "use raw html as base" but still apply pruning if configured?
                // Usually "RawHtml" implies "Full content converted to markdown without pruning".

                let markdown_result = if let Some(ref cfg) = config {
                    if matches!(cfg.content_source, Some(ContentSource::RawHtml)) {
                        // Create a generator without filter for RawHtml
                         DefaultMarkdownGenerator::new(None).generate_markdown(source_html)
                    } else {
                        generator.generate_markdown(source_html)
                    }
                } else {
                    generator.generate_markdown(source_html)
                };


                let (media, links) = if let Some(ext) = extraction {
                    (Some(ext.media), Some(ext.links))
                } else {
                    (None, None)
                };

                // Collect captured data
                let captured_requests = if capture_network {
                    Some(network_requests.lock().unwrap().clone())
                } else {
                    None
                };

                let captured_console = if capture_console {
                    Some(console_messages.lock().unwrap().clone())
                } else {
                    None
                };

                // Abort listeners (dropping handles should suffice if we want to stop background tasks,
                // but strictly speaking they might run until channel closed.
                // Since page is closing, channel closes, so tasks finish.)

                Ok(CrawlResult {
                    url: url.to_string(),
                    html,
                    success: true,
                    cleaned_html: None,
                    mhtml,
                    media,
                    links,
                    network_requests: captured_requests,
                    console_messages: captured_console,
                    screenshot: None,
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
