//! A documentation scraper that uses the FireCrawl API to extract and save content.
//! 
//! This application crawls documentation sites, extracts content, and saves it as markdown files
//! with YAML frontmatter in domain-specific directories.
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use dotenv::dotenv;
use url::Url;
use std::collections::HashSet;
use std::path::Path;

/// Represents the different actions that can be performed during web scraping.
/// 
/// Each variant corresponds to a specific action supported by the FireCrawl API.
/// Actions are serialized with a "type" field indicating the action type.
/// 
/// # Examples
/// 
/// ```
/// let actions = vec![
///     Action::Wait { milliseconds: Some(2000), selector: None },
///     Action::Click { selector: "#submit-button".to_string() },
///     Action::Screenshot { selector: Some(".content".to_string()) }
/// ];
/// ```
#[derive(Debug, Serialize)]
#[allow(dead_code)]
#[serde(tag = "type")]
pub enum Action {
    /// Wait for a specific duration or element to appear.
    /// 
    /// Either `milliseconds` or `selector` must be provided, but not both.
    /// 
    /// # Examples
    /// 
    /// ```
    /// // Wait for 2 seconds
    /// Action::Wait { milliseconds: Some(2000), selector: None }
    /// 
    /// // Wait for element to appear
    /// Action::Wait { milliseconds: None, selector: Some("#loading".to_string()) }
    /// ```
    #[serde(rename = "wait")]
    Wait {
        /// Duration to wait in milliseconds
        #[serde(skip_serializing_if = "Option::is_none")]
        milliseconds: Option<u32>,
        /// CSS selector to wait for
        #[serde(skip_serializing_if = "Option::is_none")]
        selector: Option<String>,
    },

    /// Take a screenshot of the page or a specific element.
    /// 
    /// # Examples
    /// 
    /// ```
    /// // Full page screenshot
    /// Action::Screenshot { selector: None }
    /// 
    /// // Screenshot specific element
    /// Action::Screenshot { selector: Some("#content".to_string()) }
    /// ```
    #[serde(rename = "screenshot")]
    Screenshot {
        /// Optional CSS selector for the element to screenshot
        selector: Option<String>,
    },

    /// Click on an element identified by a CSS selector.
    /// 
    /// # Examples
    /// 
    /// ```
    /// Action::Click { selector: "#submit-button".to_string() }
    /// ```
    #[serde(rename = "click")]
    Click {
        /// CSS selector for the element to click
        selector: String,
    },

    /// Write text into an input element.
    /// 
    /// # Examples
    /// 
    /// ```
    /// Action::WriteText { 
    ///     selector: "#search".to_string(),
    ///     text: "search query".to_string()
    /// }
    /// ```
    #[serde(rename = "write")]
    WriteText {
        /// CSS selector for the input element
        selector: String,
        /// Text to write into the element
        text: String,
    },

    /// Simulate pressing a keyboard key.
    /// 
    /// # Examples
    /// 
    /// ```
    /// Action::PressKey { key: "Enter".to_string() }
    /// ```
    #[serde(rename = "press")]
    PressKey {
        /// Key to simulate pressing (e.g., "Enter", "Tab", "ArrowDown")
        key: String,
    },

    /// Scroll the page by a specific number of pixels.
    /// 
    /// # Examples
    /// 
    /// ```
    /// // Scroll down 500 pixels
    /// Action::Scroll { pixels: 500 }
    /// 
    /// // Scroll up 200 pixels
    /// Action::Scroll { pixels: -200 }
    /// ```
    #[serde(rename = "scroll")]
    Scroll {
        /// Number of pixels to scroll (positive for down, negative for up)
        pixels: i32,
    },

    /// Extract content from a specific element.
    /// 
    /// # Examples
    /// 
    /// ```
    /// Action::Scrape { selector: ".article-content".to_string() }
    /// ```
    #[serde(rename = "scrape")]
    Scrape {
        /// CSS selector for the element to scrape
        selector: String,
    },

    /// Execute custom JavaScript code.
    /// 
    /// # Examples
    /// 
    /// ```
    /// Action::ExecuteJavaScript { 
    ///     script: "document.querySelector('.menu').style.display = 'none'".to_string() 
    /// }
    /// ```
    #[serde(rename = "execute")]
    ExecuteJavaScript {
        /// JavaScript code to execute
        script: String,
    },
}

/// Represents the geographical and language preferences for web scraping.
/// 
/// This struct allows specifying the country of origin for the request and
/// preferred languages for content negotiation.
/// 
/// # Examples
/// 
/// ```
/// let location = Location {
///     country: Some("AU".to_string()),  // Request from Australia
///     languages: Some(vec!["en-AU".to_string(), "en".to_string()]),
/// };
/// 
/// // Default US location with English
/// let default_location = Location {
///     country: None,  // Defaults to "US"
///     languages: Some(vec!["en".to_string()]),
/// };
/// ```
#[derive(Debug, Serialize)]
pub struct Location {
    /// The ISO 3166-1 alpha-2 country code for the request origin.
    /// 
    /// If not provided, defaults to "US". Examples: "AU", "GB", "DE".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    
    /// List of preferred languages and locales in order of priority.
    /// 
    /// Languages should be specified using IETF language tags (e.g., "en-US", "fr-FR", "de").
    /// The first language in the list has the highest priority.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub languages: Option<Vec<String>>,
}

/// Represents a request to the FireCrawl API for web scraping.
/// 
/// This struct contains all possible parameters for configuring a scraping request,
/// including content selection, behavior options, and extraction preferences.
/// 
/// # Examples
/// 
/// ```
/// let request = ScrapeRequest {
///     url: "https://example.com".to_string(),
///     formats: vec!["markdown".to_string()],
///     only_main_content: Some(true),
///     timeout: Some(30000),
///     block_ads: Some(true),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Serialize, Default)]
pub struct ScrapeRequest {
    /// The URL to scrape
    pub url: String,

    /// List of output formats to return (e.g., "markdown", "html", "links")
    pub formats: Vec<String>,

    /// Whether to extract only the main content, excluding navigation and footers
    #[serde(skip_serializing_if = "Option::is_none", rename = "onlyMainContent")]
    pub only_main_content: Option<bool>,

    /// HTML tags to include in the extraction
    #[serde(skip_serializing_if = "Option::is_none", rename = "includeTags")]
    pub include_tags: Option<Vec<String>>,

    /// HTML tags to exclude from the extraction
    #[serde(skip_serializing_if = "Option::is_none", rename = "excludeTags")]
    pub exclude_tags: Option<Vec<String>>,

    /// Custom HTTP headers for the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<serde_json::Value>,

    /// Time in milliseconds to wait before extraction
    #[serde(skip_serializing_if = "Option::is_none", rename = "waitFor")]
    pub wait_for: Option<i32>,

    /// Whether to use mobile user agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mobile: Option<bool>,

    /// Whether to skip TLS certificate verification
    #[serde(skip_serializing_if = "Option::is_none", rename = "skipTlsVerification")]
    pub skip_tls_verification: Option<bool>,

    /// Request timeout in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i32>,

    /// Options for JSON extraction
    #[serde(skip_serializing_if = "Option::is_none", rename = "jsonOptions")]
    pub json_options: Option<JsonOptions>,

    /// List of actions to perform before extraction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<Action>>,

    /// Geographical and language preferences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,

    /// Whether to remove base64 encoded images
    #[serde(skip_serializing_if = "Option::is_none", rename = "removeBase64Images")]
    pub remove_base64_images: Option<bool>,

    /// Whether to block advertisements
    #[serde(skip_serializing_if = "Option::is_none", rename = "blockAds")]
    pub block_ads: Option<bool>,
}

/// Options for JSON extraction and transformation using AI models.
/// 
/// This struct allows configuring how JSON data is extracted from web content,
/// either using a predefined schema or AI-guided extraction.
/// 
/// # Examples
/// 
/// ```
/// // Using schema-based extraction
/// let json_options = JsonOptions {
///     schema: Some(serde_json::json!({
///         "type": "object",
///         "properties": {
///             "title": { "type": "string" },
///             "price": { "type": "number" }
///         }
///     })),
///     system_prompt: None,
///     prompt: None,
/// };
/// 
/// // Using AI-guided extraction
/// let ai_options = JsonOptions {
///     schema: None,
///     system_prompt: Some("You are a product information extractor".to_string()),
///     prompt: Some("Extract product details from the content".to_string()),
/// };
/// ```
#[derive(Debug, Serialize, Default)]
pub struct JsonOptions {
    /// JSON Schema defining the structure of data to extract.
    /// 
    /// When provided, the extractor will attempt to find and structure data
    /// according to this schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,

    /// System prompt for AI-guided extraction.
    /// 
    /// Defines the AI's role and general behavior when extracting data.
    #[serde(skip_serializing_if = "Option::is_none", rename = "systemPrompt")]
    pub system_prompt: Option<String>,

    /// User prompt for AI-guided extraction.
    /// 
    /// Specific instructions for what data to extract when not using a schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

/// Response from the FireCrawl API's scrape endpoint.
/// 
/// Contains the success status of the request and the scraped data.
/// A successful response (`success = true`) will contain the requested
/// content formats in the `data` field.
/// 
/// # Examples
/// 
/// ```
/// // Successful response with markdown content
/// let response = ScrapeResponse {
///     success: true,
///     data: ScrapeData {
///         markdown: Some("# Title\nContent...".to_string()),
///         metadata: Metadata {
///             title: Some("Page Title".to_string()),
///             source_url: Some("https://example.com".to_string()),
///             ..Default::default()
///         },
///         ..Default::default()
///     },
/// };
/// ```
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ScrapeResponse {
    /// Indicates whether the scraping request was successful
    pub success: bool,
    /// Contains the scraped content and metadata
    pub data: ScrapeData,
}

/// Contains the scraped content and metadata from a web page.
/// 
/// Different content formats can be requested in the scrape request,
/// and the corresponding fields will be populated in the response.
/// The metadata field is always included.
/// 
/// # Examples
/// 
/// ```
/// let data = ScrapeData {
///     markdown: Some("# Page Title\nContent...".to_string()),
///     html: Some("<h1>Page Title</h1><p>Content...</p>".to_string()),
///     raw_html: None,
///     screenshot: None,
///     links: Some(vec![
///         "https://example.com/page1".to_string(),
///         "https://example.com/page2".to_string(),
///     ]),
///     metadata: Metadata {
///         title: Some("Page Title".to_string()),
///         source_url: Some("https://example.com".to_string()),
///         ..Default::default()
///     },
///     warning: None,
/// };
/// ```
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ScrapeData {
    /// Markdown version of the scraped content
    markdown: Option<String>,

    /// Clean HTML version of the content with unwanted elements removed
    html: Option<String>,

    /// Original HTML content of the page
    #[serde(rename = "rawHtml")]
    raw_html: Option<String>,

    /// Base64-encoded screenshot of the page or element
    screenshot: Option<String>,

    /// List of URLs found on the page
    links: Option<Vec<String>>,

    /// Metadata about the scraped page
    metadata: Metadata,

    /// Warning messages from the scraping process, if any
    warning: Option<String>,
}

/// Metadata extracted from the scraped web page.
/// 
/// Contains information about the page such as title, description,
/// language, source URL, and any errors encountered during scraping.
/// 
/// # Examples
/// 
/// ```
/// let metadata = Metadata {
///     title: Some("Page Title".to_string()),
///     description: Some("Page description for SEO".to_string()),
///     language: Some("en-US".to_string()),
///     source_url: Some("https://example.com/page".to_string()),
///     status_code: Some(200),
///     error: None,
/// };
/// 
/// // Metadata with an error
/// let error_metadata = Metadata {
///     title: None,
///     description: None,
///     language: None,
///     source_url: Some("https://example.com/404".to_string()),
///     status_code: Some(404),
///     error: Some("Page not found".to_string()),
/// };
/// ```
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
struct Metadata {
    /// Page title from the HTML <title> tag or meta tags
    title: Option<String>,

    /// Page description from meta tags
    description: Option<String>,

    /// Page language (e.g., "en-US", "fr-FR")
    language: Option<String>,

    /// Original URL of the scraped page
    #[serde(rename = "sourceURL")]
    source_url: Option<String>,

    /// HTTP status code from the page request
    #[serde(rename = "statusCode")]
    status_code: Option<i32>,

    /// Error message if scraping failed
    error: Option<String>,
}

/// Sanitizes a string for use as a filename by replacing invalid characters with underscores.
/// 
/// # Arguments
/// 
/// * `filename` - The string to sanitize
/// 
/// # Returns
/// 
/// A new string with invalid filename characters replaced by underscores
/// 
/// # Examples
/// 
/// ```
/// let safe_name = sanitize_filename("hello/world.txt");
/// assert_eq!(safe_name, "hello_world_txt");
/// 
/// let safe_name = sanitize_filename("file<with>invalid*chars");
/// assert_eq!(safe_name, "file_with_invalid_chars");
/// ```
fn sanitize_filename(filename: &str) -> String {
    let invalid_chars: &[char] = &['/', '\\', '?', '%', '*', ':', '|', '"', '<', '>', '.', ' '];
    let mut sanitized = filename.to_string();
    for c in invalid_chars {
        sanitized = sanitized.replace(*c, "_");
    }
    sanitized
}

/// Creates YAML frontmatter from metadata and adds a timestamp.
/// 
/// Generates a YAML frontmatter block containing the page title,
/// source URL, and the current UTC timestamp in ISO 8601 format.
/// 
/// # Arguments
/// 
/// * `metadata` - The metadata containing title and source URL
/// 
/// # Returns
/// 
/// A string containing the YAML frontmatter block
/// 
/// # Examples
/// 
/// ```
/// let metadata = Metadata {
///     title: Some("Page Title".to_string()),
///     source_url: Some("https://example.com".to_string()),
///     ..Default::default()
/// };
/// 
/// let frontmatter = create_frontmatter(&metadata);
/// // Results in:
/// // ---
/// // title: "Page Title"
/// // url: "https://example.com"
/// // scrapeDate: 2024-01-01T12:00:00+00:00
/// // ---
/// ```
fn create_frontmatter(metadata: &Metadata) -> String {
    let mut frontmatter = String::from("---\n");
    if let Some(title) = &metadata.title {
        frontmatter.push_str(&format!("title: \"{}\"\n", title));
    }
    if let Some(source_url) = &metadata.source_url {
        frontmatter.push_str(&format!("url: \"{}\"\n", source_url));
    }
    frontmatter.push_str(&format!("scrapeDate: {}\n", chrono::Utc::now().to_rfc3339()));
    frontmatter.push_str("---\n\n");
    frontmatter
}

/// Creates a directory based on the domain name from a URL.
/// 
/// Extracts the domain from the URL, sanitizes it for use as a directory name,
/// and creates the directory if it doesn't exist.
/// 
/// # Arguments
/// 
/// * `url` - The URL to extract the domain from
/// 
/// # Returns
/// 
/// A `Result` containing the `PathBuf` of the created directory
/// 
/// # Errors
/// 
/// Returns an error if:
/// - URL parsing fails
/// - Directory creation fails
/// 
/// # Examples
/// 
/// ```
/// let path = create_domain_directory("https://docs.example.com/page")?;
/// // Creates directory "docs_example_com" and returns its PathBuf
/// ```
fn create_domain_directory(url: &str) -> Result<PathBuf, Box<dyn Error>> {
    let parsed_url = Url::parse(url)?;
    let domain = parsed_url.domain().unwrap_or("unknown");
    let dir_name = sanitize_filename(domain);
    
    let path = PathBuf::from(&dir_name);
    fs::create_dir_all(&path)?;
    
    Ok(path)
}

/// Makes a request to the FireCrawl API with the given request body.
/// 
/// # Arguments
/// 
/// * `client` - The HTTP client
/// * `api_url` - The FireCrawl API endpoint
/// * `api_key` - The API authentication key
/// * `request` - The request body
/// 
/// # Returns
/// 
/// A `Result` containing the API response
/// 
/// # Errors
/// 
/// Returns an error if:
/// - The HTTP request fails
/// - The response status is not successful
/// - The response body cannot be parsed
/// 
/// # Examples
/// 
/// ```
/// let request = ScrapeRequest {
///     url: "https://example.com".to_string(),
///     formats: vec!["markdown".to_string()],
///     ..Default::default()
/// };
/// 
/// let response = make_api_request(&client, &api_url, &api_key, request).await?;
/// ```
async fn make_api_request(
    client: &Client,
    api_url: &str,
    api_key: &str,
    request: ScrapeRequest,
) -> Result<ScrapeResponse, Box<dyn Error>> {
    let response = client
        .post(api_url)
        .bearer_auth(api_key)
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await?;
        return Err(format!("API request failed with status {}: {}", status, error_body).into());
    }

    Ok(response.json().await?)
}

/// Extracts all documentation links from a given URL.
/// 
/// Fetches and returns a list of unique URLs from the same domain as the start URL.
/// Removes URL fragments and deduplicates the links before returning.
/// 
/// # Arguments
/// 
/// * `client` - The HTTP client
/// * `api_url` - The FireCrawl API endpoint
/// * `api_key` - The API authentication key
/// * `start_url` - The URL to extract links from
/// 
/// # Returns
/// 
/// A `Result` containing a vector of unique URLs from the same domain
/// 
/// # Errors
/// 
/// Returns an error if:
/// - The API request fails
/// - URL parsing fails
/// - The response cannot be processed
/// 
/// # Examples
/// 
/// ```
/// let links = extract_doc_links(&client, &api_url, &api_key, "https://docs.example.com").await?;
/// // Returns: ["https://docs.example.com/page1", "https://docs.example.com/page2"]
/// ```
async fn extract_doc_links(
    client: &Client,
    api_url: &str,
    api_key: &str,
    start_url: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let request = ScrapeRequest {
        url: start_url.to_string(),
        formats: vec!["links".to_string()],
        ..Default::default()
    };

    let scrape_response = make_api_request(client, api_url, api_key, request).await?;
    let base_url = Url::parse(start_url).map_err(|e| format!("Failed to parse start URL: {}", e))?;
    let base_domain = base_url.domain().ok_or("Invalid base domain")?;

    Ok(scrape_response.data.links
        .unwrap_or_default()
        .into_iter()
        .filter_map(|link| {
            Url::parse(&link).ok().and_then(|mut url| {
                if url.domain() == Some(base_domain) {
                    url.set_fragment(None);
                    Some(url.to_string())
                } else {
                    None
                }
            })
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect())
}

/// Scrapes documentation from a website and saves it as markdown files.
/// 
/// Downloads content from all pages on the same domain as the start URL,
/// converts them to markdown format, and saves them with YAML frontmatter.
/// 
/// # Arguments
/// 
/// * `client` - The HTTP client
/// * `api_url` - The FireCrawl API endpoint
/// * `api_key` - The API authentication key
/// * `start_url` - The URL to start scraping from
/// 
/// # Returns
/// 
/// A `Result` indicating success or failure of the scraping operation
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Directory creation fails
/// - Link extraction fails
/// - API requests fail
/// - File writing fails
/// 
/// # Examples
/// 
/// ```
/// scrape_documentation(&client, &api_url, &api_key, "https://docs.example.com").await?;
/// // Creates markdown files in a directory named after the domain
/// ```
async fn scrape_documentation(
    client: &Client,
    api_url: &str,
    api_key: &str,
    start_url: &str,
) -> Result<(), Box<dyn Error>> {
    let output_dir = create_domain_directory(start_url)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;
    println!("Saving files to: {}", output_dir.display());

    let doc_urls = extract_doc_links(client, api_url, api_key, start_url).await?;
    println!("Found {} documentation pages", doc_urls.len());

    for url in doc_urls {
        let result = process_page(client, api_url, api_key, &url, &output_dir).await;
        
        if let Err(e) = result {
            eprintln!("Error processing {}: {}", url, e);
            continue; // Continue with next URL on error
        }
    }

    Ok(())
}

/// Processes a single documentation page and saves it as markdown.
/// 
/// # Arguments
/// 
/// * `client` - The HTTP client
/// * `api_url` - The FireCrawl API endpoint
/// * `api_key` - The API authentication key
/// * `url` - The URL to process
/// * `output_dir` - Directory to save the markdown file
/// 
/// # Returns
/// 
/// A `Result` indicating success or failure of the page processing
/// 
/// # Errors
/// 
/// Returns an error if:
/// - The API request fails
/// - File writing fails
/// 
/// # Examples
/// 
/// ```
/// process_page(&client, &api_url, &api_key, "https://docs.example.com/page", &path).await?;
/// ```
async fn process_page(
    client: &Client,
    api_url: &str,
    api_key: &str,
    url: &str,
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let request = ScrapeRequest {
        url: url.to_string(),
        formats: vec!["markdown".to_string()],
        ..Default::default()
    };

    let scrape_response = make_api_request(client, api_url, api_key, request).await?;
    
    let filename = match &scrape_response.data.metadata.title {
        Some(title) => format!("{}.md", sanitize_filename(title)),
        None => format!("page_{}.md", sanitize_filename(url)),
    };

    let file_path = output_dir.join(filename);

    if let Some(markdown) = &scrape_response.data.markdown {
        let content = format!(
            "{}{}",
            create_frontmatter(&scrape_response.data.metadata),
            markdown
        );
        
        fs::write(&file_path, &content)
            .map_err(|e| format!("Failed to write file {}: {}", file_path.display(), e))?;
        println!("Saved: {}", file_path.display());
    } else {
        eprintln!("No markdown content received for {}", url);
    }

    if let Some(warning) = &scrape_response.data.warning {
        eprintln!("Warning for {}: {}", url, warning);
    }

    Ok(())
}

/// Scrapes documentation from a website and saves it as markdown files.
/// 
/// Environment variables:
/// - FIRECRAWL_API_URL: Optional. Defaults to "https://api.firecrawl.dev"
/// - FIRECRAWL_API_KEY: Required. Your API authentication key
/// 
/// Usage: cargo run -- <url>
/// Example: cargo run -- https://docs.example.com
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load environment variables
    dotenv().ok();

    // Initialize HTTP client
    let client = Client::new();

    // Get API configuration
    let api_url = format!("{}/v1/scrape", 
        std::env::var("FIRECRAWL_API_URL")
            .unwrap_or_else(|_| "https://api.firecrawl.dev".to_string())
    );
    
    let api_key = std::env::var("FIRECRAWL_API_KEY")
        .map_err(|_| "FIRECRAWL_API_KEY must be set in .env file")?;

    // Parse command line arguments
    let start_url = std::env::args()
        .nth(1)
        .ok_or("Usage: cargo run -- <url>")?;

    // Run the scraper
    scrape_documentation(&client, &api_url, &api_key, &start_url).await?;

    Ok(())
}
