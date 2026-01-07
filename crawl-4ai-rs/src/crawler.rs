use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::target::{CreateBrowserContextParams, CreateTargetParams};
use chromiumoxide::cdp::browser_protocol::browser::BrowserContextId;
use futures::StreamExt;
use anyhow::{Result, anyhow};
use crate::models::{CrawlResult, MediaItem, Link, CrawlerRunConfig, WaitStrategy};
use crate::markdown::DefaultMarkdownGenerator;
use crate::content_filter::PruningContentFilter;
use std::env;
use std::path::Path;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::Deserialize;

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
            return Ok(());
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
        if self.browser.is_none() {
            self.start().await?;
        }

        let browser = self.browser.as_ref().unwrap();

        let page = if let Some(ref cfg) = config {
            if let Some(ref session_id) = cfg.session_id {
                let context_id = if let Some(id) = self.sessions.get(session_id) {
                    id.clone()
                } else {
                    let id = browser.create_browser_context(CreateBrowserContextParams::default()).await?;
                    self.sessions.insert(session_id.clone(), id.clone());
                    id
                };

                let params = CreateTargetParams::builder()
                    .url(url)
                    .browser_context_id(context_id)
                    .build()
                    .map_err(|e| anyhow!(e))?;

                browser.new_page(params).await?
            } else {
                browser.new_page(url).await?
            }
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
                        // For data URLs, hostname is empty string, so everything usually counts as external
                        // unless we handle it specifically.
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

        // If a session ID is used, we might want to keep the page or context open.
        // For now, we close the page, but the context persists in self.sessions.
        // The user requirement says "reusing browser contexts/sessions".
        // Typically sessions persist cookies/storage. Closing the page doesn't destroy the context.
        page.close().await?;

        // Generate Markdown
        let content_filter = PruningContentFilter::default();
        let generator = DefaultMarkdownGenerator::new(Some(content_filter));
        let markdown_result = generator.generate_markdown(&html);

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
            screenshot: None,
            markdown: Some(markdown_result),
            extracted_content: None,
            error_message: None,
        })
    }
}
