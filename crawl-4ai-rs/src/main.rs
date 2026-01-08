use clap::{Parser, ValueEnum};
use crawl_4ai_rs::crawler::AsyncWebCrawler;
use crawl_4ai_rs::models::{CrawlerRunConfig, CrawlResult};
use std::fs;
use std::path::PathBuf;
use anyhow::Result;
use log::{info, error};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The URL to crawl
    #[arg(required = true)]
    url: String,

    /// Output file path (optional)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,

    /// Take a screenshot
    #[arg(long, default_value_t = false)]
    screenshot: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum OutputFormat {
    Markdown,
    Json,
    RawHtml,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    info!("Starting crawl for URL: {}", args.url);

    let mut crawler = AsyncWebCrawler::new();

    let config = CrawlerRunConfig {
        screenshot: args.screenshot,
        ..Default::default()
    };

    let result = crawler.arun(&args.url, Some(config)).await;

    match result {
        Ok(crawl_result) => {
            info!("Crawl successful!");
            handle_output(crawl_result, &args)?;
        }
        Err(e) => {
            error!("Crawl failed: {}", e);
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn handle_output(result: CrawlResult, args: &Args) -> Result<()> {
    let content = match args.format {
        OutputFormat::Markdown => result
            .markdown
            .clone()
            .map(|m| m.raw_markdown)
            .unwrap_or_else(|| String::from("No markdown generated")),
        OutputFormat::Json => serde_json::to_string_pretty(&result)?,
        OutputFormat::RawHtml => result.html.clone(),
    };

    if let Some(path) = &args.output {
        fs::write(path, content)?;
        println!("Output written to {:?}", path);
    } else {
        println!("{}", content);
    }

    if args.screenshot {
       if let Some(screenshot_data) = result.screenshot {
           if let Some(path) = &args.output {
               // If output is a file, save screenshot with .png extension
               let mut screenshot_path = path.clone();
               screenshot_path.set_extension("png");

               // Attempt to decode base64
               use base64::{Engine as _, engine::general_purpose};
               match general_purpose::STANDARD.decode(&screenshot_data) {
                   Ok(bytes) => {
                       if let Err(e) = fs::write(&screenshot_path, bytes) {
                           error!("Failed to save screenshot: {}", e);
                       } else {
                           info!("Screenshot saved to {:?}", screenshot_path);
                       }
                   }
                   Err(e) => {
                        error!("Failed to decode screenshot base64: {}", e);
                   }
               }
           } else {
               info!("Screenshot captured but no output file specified to derive filename from.");
           }
       } else {
           info!("Screenshot requested but none returned.");
       }
    }

    Ok(())
}
