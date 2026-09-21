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

/// Many sites answer bots with an error page; a normal browser identity gets the article.
const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Safari/537.36";

/// A client that talks to exactly one address: the one that was checked. Redirects are followed by
/// hand ([`fetch`]), because a redirect target needs the same check — and a second DNS lookup could
/// answer differently than the one that was validated (DNS rebinding).
fn pinned_client(host: &str, address: std::net::SocketAddr) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .resolve(host, address)
        .build()
        .map_err(|e| t!("page.failed", error = e).to_string())
}

pub async fn fetch(url: &str) -> Result<Page, String> {
    let mut target = web_url(url)?;
    let mut response = None;
    // Every hop is checked and pinned on its own.
    for _ in 0..=MAX_REDIRECTS {
        let address = ensure_public(&target).await?;
        let host = target.host_str().unwrap_or_default().to_string();
        let answer = pinned_client(&host, address)?
            .get(target.clone())
            .timeout(TIMEOUT)
            .send()
            .await
            .map_err(|e| t!("page.failed", error = e.without_url()).to_string())?;

        match redirect_target(&answer, &target) {
            Some(next) => target = web_url(next.as_str())?,
            None => {
                response = Some(answer);
                break;
            }
        }
    }
    let Some(mut response) = response else {
        return Err(t!("page.failed", error = "too many redirects").into());
    };

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
    let final_url = target.to_string();

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
    let raw = decode(&body, &content_type);

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

/// A web address, or an error naming the input.
fn web_url(url: &str) -> Result<url::Url, String> {
    let parsed = url::Url::parse(url.trim()).map_err(|_| t!("page.invalid_url", url = url).to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(t!("page.invalid_url", url = url).into());
    }
    Ok(parsed)
}

/// Where a redirect answer points, as an absolute address.
fn redirect_target(response: &reqwest::Response, current: &url::Url) -> Option<url::Url> {
    if !response.status().is_redirection() {
        return None;
    }
    let location = response.headers().get(reqwest::header::LOCATION)?.to_str().ok()?;
    current.join(location).ok()
}

/// Checks the address and every IP its host name resolves to, and returns the one to talk to.
async fn ensure_public(url: &url::Url) -> Result<std::net::SocketAddr, String> {
    let refused = || t!("page.private", url = url.as_str()).to_string();
    if host_is_private(url) {
        return Err(refused());
    }
    let Some(host) = url.host_str() else {
        return Err(refused());
    };
    let port = url.port_or_known_default().unwrap_or(443);
    // A literal address was already checked above.
    if let Ok(ip) = host.trim_matches(['[', ']']).parse::<IpAddr>() {
        return Ok(std::net::SocketAddr::new(ip, port));
    }
    let addresses: Vec<std::net::SocketAddr> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| t!("page.failed", error = e).to_string())?
        .collect();
    if addresses.iter().any(|address| is_private_ip(address.ip())) {
        return Err(refused());
    }
    addresses
        .into_iter()
        .next()
        .ok_or_else(|| t!("page.failed", error = "no address").to_string())
}

/// Text of a response body. Pages that say they are Latin-1 (still common on older German sites)
/// are decoded as Windows-1252, everything else as UTF-8.
fn decode(body: &[u8], content_type: &str) -> String {
    // The value ends at the first character that cannot be part of a charset name.
    let name = |after: &str| -> String {
        after
            .trim()
            .trim_start_matches(['"', '\''])
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect::<String>()
            .to_ascii_lowercase()
    };
    let charset = content_type
        .split("charset=")
        .nth(1)
        .map(name)
        .filter(|charset| !charset.is_empty())
        .or_else(|| {
            // No header charset: look at the first bytes, where a <meta charset> would be.
            let head = String::from_utf8_lossy(&body[..body.len().min(2048)]).into_owned();
            head.split("charset=").nth(1).map(name)
        })
        .unwrap_or_default();

    if matches!(
        charset.as_str(),
        "iso-8859-1" | "iso8859-1" | "latin1" | "windows-1252" | "cp1252"
    ) {
        body.iter().map(|byte| windows_1252(*byte)).collect()
    } else {
        String::from_utf8_lossy(body).into_owned()
    }
}

/// Windows-1252 differs from Latin-1 only in 0x80–0x9F (quotes, dashes, €).
fn windows_1252(byte: u8) -> char {
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}', '\u{90}', '‘',
        '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
    ];
    match byte {
        0x80..=0x9f => HIGH[(byte - 0x80) as usize],
        other => char::from(other),
    }
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
            let [a, b, ..] = ip.octets();
            ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_broadcast()
                // Carrier-grade NAT (100.64.0.0/10), also used by Tailscale.
                || (a == 100 && (64..128).contains(&b))
                // IETF protocol assignments, benchmarking and the reserved top range.
                || (a == 192 && b == 0 && ip.octets()[2] == 0)
                || (a == 198 && (18..20).contains(&b))
                || a >= 240
        }
        IpAddr::V6(ip) => {
            if let Some(v4) = ip.to_ipv4_mapped() {
                return is_private_ip(IpAddr::V4(v4));
            }
            let segments = ip.segments();
            let first = segments[0];
            // NAT64 (64:ff9b::/96) carries an IPv4 address that must be checked itself.
            if segments[0] == 0x0064 && segments[1] == 0xff9b {
                let last = u32::from(segments[6]) << 16 | u32::from(segments[7]);
                return is_private_ip(IpAddr::V4(std::net::Ipv4Addr::from(last)));
            }
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
        Regex::new(
            r"(?i)</?(p|div|section|article|main|header|h[1-6]|ul|ol|tr|table|blockquote|pre|br|hr|dt|dd)\b[^>]*>",
        )
        .unwrap()
    });
    static HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<h[1-6]\b[^>]*>").unwrap());
    static ITEM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<li\b[^>]*>").unwrap());
    // Quoted attribute values may contain ">" (e.g. JSON in data attributes).
    // Only double-quoted attribute values may hide a ">"; a lone apostrophe in page text
    // ("don't") must not swallow everything up to the next one.
    static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"<(?:[^>"]|"[^"]*")*>"#).unwrap());

    let cleaned = DROP.iter().fold(html.to_string(), |text, pattern| {
        pattern.replace_all(&text, " ").into_owned()
    });
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
    fn latin1_pages_are_decoded_by_their_charset() {
        // "Grüße" in Windows-1252, plus the euro sign that plain Latin-1 does not have.
        let body = [b'G', b'r', 0xfc, 0xdf, b'e', b' ', 0x80];
        assert_eq!(decode(&body, "text/html; charset=iso-8859-1"), "Grüße €");
        assert_eq!(decode(&body, "text/html; charset=\"windows-1252\""), "Grüße €");
        // Without a charset the bytes are read as UTF-8 (here: not valid, so replaced).
        assert!(decode(&body, "text/html").contains("Gr"));
        assert_eq!(decode("Grüße".as_bytes(), "text/html; charset=utf-8"), "Grüße");
        // A page that only says it in its <meta> tag.
        let meta = [b"<meta charset=\"iso-8859-1\">Gr".as_slice(), &[0xfc], b"".as_slice()].concat();
        assert!(decode(&meta, "text/html").ends_with("Grü"));
    }

    #[test]
    fn apostrophes_in_text_do_not_swallow_the_page() {
        let html = "<p>It doesn't matter</p><p>Second sentence</p>";
        assert_eq!(html_to_text(html), "It doesn't matter\n\nSecond sentence");
    }

    #[test]
    fn more_reserved_ranges_are_refused() {
        let private = |ip: &str| is_private_ip(ip.parse().unwrap());
        assert!(private("198.18.0.1"), "benchmark range");
        assert!(private("240.0.0.1"), "reserved range");
        assert!(private("192.0.0.1"), "IETF assignments");
        assert!(private("64:ff9b::7f00:1"), "NAT64 wrapping 127.0.0.1");
        assert!(!private("64:ff9b::0808:0808"), "NAT64 wrapping 8.8.8.8");
        assert!(!private("93.184.216.34"));
    }
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
        assert_eq!(
            decode_entities("a &lt;b&gt; &#x27;c&#39; &uuml; &unknown;"),
            "a <b> 'c' ü &unknown;"
        );
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
        let page = fetch("https://en.wikipedia.org/wiki/Rust_(programming_language)")
            .await
            .unwrap();
        assert!(
            page.text.contains("Rust") && page.title.contains("Rust"),
            "{}",
            page.text
        );
        println!(
            "{}
{}",
            page.title,
            &page.text[..page.text.len().min(600)]
        );
    }
}
