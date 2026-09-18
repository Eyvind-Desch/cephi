//! Reads a web page as plain text for the model (`fetch_page`), e.g. a search result whose excerpt
//! was not enough.
//!
//! Only public http(s) addresses: the model must not be able to reach services on this PC or in the
//! local network (Ollama, SearXNG, the router, …), also not through a redirect.

use std::net::IpAddr;
use std::sync::LazyLock;
use std::time::Duration;

use regex::Regex;
use rust_i18n::t;

const TIMEOUT: Duration = Duration::from_secs(12);
/// Larger pages are cut; the text is shortened for the model anyway.
const MAX_BYTES: usize = 3 * 1024 * 1024;
const MAX_REDIRECTS: usize = 5;
/// Characters of page text handed on (the agent shortens tool output further if configured lower).
pub const MAX_TEXT_CHARS: usize = 12_000;

pub struct Page {
    /// Final address after redirects.
    pub url: String,
    pub title: String,
    pub text: String,
}

/// A client that refuses redirects to private addresses. Built once; reqwest clients are cheap to clone.
fn client() -> &'static reqwest::Client {
    static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
        reqwest::Client::builder()
            // Many sites answer bots with an error page; a normal browser identity gets the article.
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Safari/537.36")
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= MAX_REDIRECTS {
                    attempt.error("too many redirects")
                } else if host_is_private(attempt.url()) {
                    attempt.error("redirect to a private address")
                } else {
                    attempt.follow()
                }
            }))
            .build()
            .unwrap_or_default()
    });
    &CLIENT
}

pub async fn fetch(url: &str) -> Result<Page, String> {
    let parsed = url::Url::parse(url.trim()).map_err(|_| t!("page.invalid_url", url = url).to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(t!("page.invalid_url", url = url).into());
    }
    ensure_public(&parsed).await?;

    let mut response = client()
        .get(parsed.clone())
        .timeout(TIMEOUT)
        .send()
        .await
        .map_err(|e| t!("page.failed", error = e.without_url()).to_string())?;
    if !response.status().is_success() {
        return Err(t!("page.failed", error = response.status()).into());
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("text/html")
        .to_ascii_lowercase();
    if !(content_type.contains("html") || content_type.starts_with("text/")) {
        return Err(t!("page.not_text", kind = content_type).into());
    }
    let final_url = response.url().to_string();

    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| t!("page.failed", error = e.without_url()).to_string())?
    {
        body.extend_from_slice(&chunk);
        if body.len() >= MAX_BYTES {
            break;
        }
    }
    let raw = String::from_utf8_lossy(&body);

    let (title, text) = if content_type.contains("html") {
        (html_title(&raw), html_to_text(&raw))
    } else {
        (String::new(), raw.trim().to_string())
    };
    if text.trim().is_empty() {
        return Err(t!("page.empty").into());
    }
    Ok(Page {
        url: final_url,
        title,
        text: shorten(&text, MAX_TEXT_CHARS),
    })
}

/// Checks the address and every IP its host name resolves to.
async fn ensure_public(url: &url::Url) -> Result<(), String> {
    let refused = || t!("page.private", url = url.as_str()).to_string();
    if host_is_private(url) {
        return Err(refused());
    }
    let Some(host) = url.host_str() else { return Err(refused()) };
    if host.parse::<IpAddr>().is_ok() || host.starts_with('[') {
        return Ok(());
    }
    let port = url.port_or_known_default().unwrap_or(443);
    let addresses = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| t!("page.failed", error = e).to_string())?;
    for address in addresses {
        if is_private_ip(address.ip()) {
            return Err(refused());
        }
    }
    Ok(())
}

/// Without DNS: literal private IPs and local host names.
fn host_is_private(url: &url::Url) -> bool {
    match url.host() {
        Some(url::Host::Ipv4(ip)) => is_private_ip(IpAddr::V4(ip)),
        Some(url::Host::Ipv6(ip)) => is_private_ip(IpAddr::V6(ip)),
        Some(url::Host::Domain(domain)) => {
            let domain = domain.trim_end_matches('.').to_ascii_lowercase();
            domain == "localhost"
                || domain.ends_with(".localhost")
                || domain.ends_with(".local")
                || domain.ends_with(".internal")
                || domain.ends_with(".lan")
                || domain.ends_with(".home")
                // A single label ("router", "nas") is a machine in the local network.
                || !domain.contains('.')
        }
        None => true,
    }
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_broadcast()
                // Carrier-grade NAT (100.64.0.0/10), also used by Tailscale.
                || (ip.octets()[0] == 100 && (64..128).contains(&ip.octets()[1]))
        }
        IpAddr::V6(ip) => {
            if let Some(v4) = ip.to_ipv4_mapped() {
                return is_private_ip(IpAddr::V4(v4));
            }
            let first = ip.segments()[0];
            ip.is_loopback()
                || ip.is_unspecified()
                // Unique local (fc00::/7) and link-local (fe80::/10).
                || (first & 0xfe00) == 0xfc00
                || (first & 0xffc0) == 0xfe80
        }
    }
}

fn html_title(html: &str) -> String {
    static TITLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<title[^>]*>(.*?)</title>").unwrap());
    TITLE
        .captures(html)
        .map(|c| decode_entities(c[1].trim()))
        .map(|t| t.split_whitespace().collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
}

/// Readable text of a page: the article or main part if marked, without scripts, menus and footers.
pub fn html_to_text(html: &str) -> String {
    // One pattern per element: the regex crate has no backreferences to match the closing tag.
    static DROP: LazyLock<Vec<Regex>> = LazyLock::new(|| {
        let elements = [
            "script", "style", "noscript", "svg", "template", "iframe", "head", "nav", "footer", "aside", "form",
            "button", "select",
        ];
        elements
            .iter()
            .map(|e| Regex::new(&format!(r"(?is)<{e}\b.*?</\s*{e}\s*>")).unwrap())
            .chain([
                Regex::new(r"(?s)<!--.*?-->").unwrap(),
                // List items that are nothing but a short link are menus (languages, table of contents, …).
                Regex::new(r"(?is)<li\b[^>]*>\s*(?:<(?:span|div|p)\b[^>]*>\s*)*<a\b[^>]*>(?:\s*<[^>]*>)*[^<]{0,60}(?:<[^>]*>\s*)*</a>\s*(?:</(?:span|div|p)\s*>\s*)*</li>").unwrap(),
            ])
            .collect()
    });
    static MAIN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<(article|main)\b[^>]*>(.*)</\s*(?:article|main)\s*>").unwrap());
    static BLOCK: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)</?(p|div|section|article|main|header|h[1-6]|ul|ol|tr|table|blockquote|pre|br|hr|dt|dd)\b[^>]*>").unwrap()
    });
    static HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<h[1-6]\b[^>]*>").unwrap());
    static ITEM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<li\b[^>]*>").unwrap());
    // Quoted attribute values may contain ">" (e.g. JSON in data attributes).
    static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"<(?:[^>"']|"[^"]*"|'[^']*')*>"#).unwrap());

    let cleaned = DROP
        .iter()
        .fold(html.to_string(), |text, pattern| pattern.replace_all(&text, " ").into_owned());
    // Prefer the marked-up content; fall back to the whole body.
    let content = MAIN
        .captures(&cleaned)
        .map(|c| c[2].to_string())
        .filter(|main| TAG.replace_all(main, "").trim().len() > 200)
        .unwrap_or_else(|| cleaned.to_string());

    let marked = HEADING.replace_all(&content, "\n\n## ");
    let marked = ITEM.replace_all(&marked, "\n- ");
    let marked = BLOCK.replace_all(&marked, "\n");
    let plain = decode_entities(&TAG.replace_all(&marked, ""));

    // Collapse whitespace per line and blank-line runs.
    let mut text = String::new();
    let mut blank = true;
    for line in plain.lines() {
        let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if line.is_empty() || line == "-" || line == "##" {
            if !blank {
                text.push('\n');
                blank = true;
            }
        } else {
            text.push_str(&line);
            text.push('\n');
            blank = false;
        }
    }
    text.trim().to_string()
}

fn decode_entities(text: &str) -> String {
    static ENTITY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"&(#x[0-9a-fA-F]+|#[0-9]+|[a-zA-Z]+);").unwrap());
    ENTITY
        .replace_all(text, |caps: &regex::Captures| {
            let entity = &caps[1];
            let decoded = if let Some(hex) = entity.strip_prefix("#x").or_else(|| entity.strip_prefix("#X")) {
                u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
            } else if let Some(decimal) = entity.strip_prefix('#') {
                decimal.parse().ok().and_then(char::from_u32)
            } else {
                match entity {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    "nbsp" => Some(' '),
                    "ndash" => Some('–'),
                    "mdash" => Some('—'),
                    "hellip" => Some('…'),
                    "laquo" => Some('«'),
                    "raquo" => Some('»'),
                    "bdquo" => Some('„'),
                    "ldquo" => Some('“'),
                    "rdquo" => Some('”'),
                    "lsquo" => Some('‘'),
                    "rsquo" => Some('’'),
                    "auml" => Some('ä'),
                    "ouml" => Some('ö'),
                    "uuml" => Some('ü'),
                    "Auml" => Some('Ä'),
                    "Ouml" => Some('Ö'),
                    "Uuml" => Some('Ü'),
                    "szlig" => Some('ß'),
                    "euro" => Some('€'),
                    "copy" => Some('©'),
                    _ => None,
                }
            };
            decoded.map_or_else(|| caps[0].to_string(), String::from)
        })
        .into_owned()
}

fn shorten(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let cut: String = text.chars().take(max_chars).collect();
    format!("{} …", cut.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_the_article_text() {
        let html = r#"<html><head><title>Tauri &amp; Rust</title><style>p{}</style></head><body>
            <nav><a href="/">Home</a> <a href="/docs">Docs</a></nav>
            <main><h1>What is Tauri?</h1><p>Tauri is a <b>framework</b> for building&nbsp;tiny apps. It uses the
            system webview and Rust for the backend, so apps stay small and fast. Many developers use it
            instead of Electron because of the size difference.</p><ul><li>Small</li><li>Fast</li></ul>
            <script>alert("x")</script></main><footer>© 2026</footer></body></html>"#;
        assert_eq!(html_title(html), "Tauri & Rust");
        let text = html_to_text(html);
        assert!(text.starts_with("## What is Tauri?"), "{text}");
        assert!(text.contains("Tauri is a framework for building tiny apps."));
        assert!(text.contains("- Small\n- Fast"));
        assert!(!text.contains("Home") && !text.contains("alert") && !text.contains("2026"));
    }

    #[test]
    fn decodes_entities() {
        assert_eq!(decode_entities("a &lt;b&gt; &#x27;c&#39; &uuml; &unknown;"), "a <b> 'c' ü &unknown;");
    }

    #[test]
    fn refuses_local_addresses() {
        let private = |u: &str| host_is_private(&url::Url::parse(u).unwrap());
        assert!(private("http://localhost:11434/api/tags"));
        assert!(private("http://127.0.0.1:8888/"));
        assert!(private("http://192.168.178.1/"));
        assert!(private("http://[::1]/"));
        assert!(private("http://router/"));
        assert!(private("http://nas.local/"));
        assert!(private("http://100.101.1.1/"));
        assert!(!private("https://de.wikipedia.org/wiki/Tauri"));
        assert!(!private("https://1.1.1.1/"));
    }

    #[tokio::test]
    async fn refuses_non_web_schemes_and_private_hosts() {
        assert!(fetch("file:///C:/Windows/win.ini").await.is_err());
        assert!(fetch("http://127.0.0.1:1/").await.is_err());
    }

    /// Needs internet.
    #[tokio::test]
    #[ignore]
    async fn live_fetch() {
        let page = fetch("https://en.wikipedia.org/wiki/Rust_(programming_language)").await.unwrap();
        assert!(page.text.contains("Rust") && page.title.contains("Rust"), "{}", page.text);
        println!("{}
{}", page.title, &page.text[..page.text.len().min(600)]);
    }
}
