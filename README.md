# Documentation Scraper

A Rust-based documentation scraper that uses the FireCrawl API to extract and save content from documentation websites as markdown files.

## Features

- Extracts content from documentation websites
- Converts HTML to clean markdown format
- Maintains original document structure
- Saves metadata in YAML frontmatter
- Handles pagination and navigation
- Supports custom HTTP headers and timeouts
- Includes error handling and retry logic
- Creates organized directory structure by domain
- Supports AI-guided content extraction
- Handles base64 image removal
- Provides ad and cookie popup blocking
- Supports mobile device emulation
- Allows custom JavaScript execution
- Includes geographical location spoofing

## Installation

1. Clone the repository:
```bash
git clone https://github.com/astrazds/scraper
cd scraper
```

2. Create a `.env` file in the project root:
```bash
FIRECRAWL_API_KEY=your-api-key-here
FIRECRAWL_API_URL=https://api.firecrawl.dev  # Optional, defaults to this value
```

3. Build the project:
```bash
cargo build --release
```

## Usage

### Basic Usage

Run the scraper with a starting URL:

```bash
cargo run -- https://docs.example.com
```

### What It Does

1. Creates a directory named after the domain (e.g., `docs_example_com`)
2. Extracts all documentation links from the starting URL
3. Downloads and converts each page to markdown
4. Saves files with YAML frontmatter containing metadata

### Output Format

Each markdown file includes:
```markdown
---
title: "Page Title"
url: "https://docs.example.com/page"
scrapeDate: 2024-01-01T12:00:00+00:00
---

# Page Content
...
```

## Configuration

### Environment Variables

- `FIRECRAWL_API_KEY` (required): Your FireCrawl API authentication key
- `FIRECRAWL_API_URL` (optional): Custom API endpoint, defaults to `https://api.firecrawl.dev`

### Advanced Options

The scraper supports various FireCrawl API features:
- Custom HTTP headers
- JavaScript execution
- Mobile device emulation
- Geographical location spoofing
- Ad and cookie popup blocking
- Base64 image removal
- Custom timeouts and wait conditions

## Error Handling

The scraper includes comprehensive error handling:
- API request failures
- Network issues
- File system errors
- Invalid URLs
- Missing content
- Rate limiting

Failed page scrapes are logged but don't stop the entire process.

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Support

- For FireCrawl API questions: Visit their [support page](https://docs.firecrawl.dev/support)
- For scraper issues: Open an issue on GitHub

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.