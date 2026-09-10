//! HTML parsing helpers.
//!
//! Wraps the subset of Symfony DomCrawler that the jikan-php parsers use, so
//! that XPath expressions can be copied from the PHP almost verbatim:
//!
//! ```ignore
//! let doc = HtmlDoc::parse(html)?;
//! let title = doc.attr("//meta[@property='og:title']", "content")?;
//! let score = doc.text("//span[@itemprop='ratingValue']")?;
//! doc.nodes("//div[@class='spaceit_pad']")?.iter().for_each(...);
//! ```
//!
//! The wrappers mirror DomCrawler semantics:
//!
//! - `filterXPath()` relativizes leading `//` to `descendant-or-self::` and
//!   evaluates per context node (ported byte-for-byte from
//!   `Crawler::relativize()`);
//! - `attr()`/`text()`/`html()` operate on the first matched node;
//! - `text()` normalizes whitespace like `Crawler::normalizeWhitespace()`;
//! - `filter()` (CSS) is translated to XPath with the same predicates as
//!   Symfony's CssSelector component.
//!
//! All XPath evaluation failures surface as [`ParseError::InvalidXPath`]; a
//! missing match is `Ok(None)`/`Ok(vec![])` where DomCrawler would throw
//! `InvalidArgumentException` on an empty crawler.
//!
//! Documents are libxml2 trees (`Rc<RefCell<..>>` internally) and therefore
//! `!Send`/`!Sync`: parse and consume them synchronously, do not hold an
//! [`HtmlDoc`] across an `.await`.

use std::sync::OnceLock;

use libxml::parser::Parser;
use libxml::tree::{Document, Node};
use libxml::xpath::Context;
use regex::bytes::Regex as BytesRegex;
use regex::Regex;
use serde_json::{json, Value};

use crate::error::ParseError;

/// An HTML document parsed by libxml2 plus its root element.
#[derive(Clone)]
pub struct HtmlDoc {
    document: Document,
    root: Node,
}

impl std::fmt::Debug for HtmlDoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HtmlDoc")
            .field("root", &self.root.get_name())
            .finish()
    }
}

/// A single libxml2 node bound to its document.
///
/// `Clone` is cheap (refcounted) and `nodes()` returns a `Vec<HtmlNode>` that
/// can be indexed/sliced like the PHP crawler list.
#[derive(Clone)]
pub struct HtmlNode {
    document: Document,
    node: Node,
}

impl std::fmt::Debug for HtmlNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HtmlNode")
            .field("name", &self.node.get_name())
            .finish()
    }
}

impl HtmlDoc {
    /// Parse HTML bytes. Mirrors `new Crawler($html)`:
    ///
    /// 1. charset is detected like `Crawler::addContent()` (valid UTF-8 else
    ///    ISO-8859-1, then the first `<meta charset=...>` wins);
    /// 2. the bytes are transcoded to UTF-8;
    /// 3. libxml2's HTML parser builds the tree (same parser PHP's
    ///    `DOMDocument::loadHTML` uses).
    pub fn parse<B: AsRef<[u8]>>(bytes: B) -> Result<Self, ParseError> {
        let bytes = bytes.as_ref();
        let decoded = decode_html(bytes);

        // Symmetric with Symfony's `parseXhtml()`: force the encoding with an
        // XML declaration instead of `htmlReadMemory`'s encoding argument.
        // (The libxml crate's `parse_string_with_options` passes a dangling
        // pointer for `ParserOptions::encoding`, which makes parsing
        // nondeterministic; the declaration also wins over `<meta charset>`.)
        let input = format!("<?xml encoding=\"UTF-8\">{decoded}");
        let parser = Parser::default_html();
        let document = parser
            .parse_string(input.as_bytes())
            .map_err(|e| ParseError::Html(e.to_string()))?;
        let root = document.get_root_element().ok_or(ParseError::MissingRoot)?;
        Ok(HtmlDoc { document, root })
    }

    /// Parse a UTF-8 string (convenience for tests and fixtures).
    pub fn parse_str(html: &str) -> Result<Self, ParseError> {
        HtmlDoc::parse(html.as_bytes())
    }

    /// The libxml2 document.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// The root element (`<html>` for full pages) as an [`HtmlNode`].
    pub fn root(&self) -> HtmlNode {
        HtmlNode {
            document: self.document.clone(),
            node: self.root.clone(),
        }
    }

    /// `$crawler->filterXPath($xpath)`.
    pub fn nodes(&self, xpath: &str) -> Result<Vec<HtmlNode>, ParseError> {
        let nodes = eval_nodes(&self.document, std::slice::from_ref(&self.root), xpath)?;
        Ok(wrap_nodes(&self.document, nodes))
    }

    /// `$crawler->filterXPath($xpath)->first()`.
    pub fn first(&self, xpath: &str) -> Result<Option<HtmlNode>, ParseError> {
        Ok(self.nodes(xpath)?.into_iter().next())
    }

    /// `$crawler->filterXPath($xpath)->last()`.
    pub fn last(&self, xpath: &str) -> Result<Option<HtmlNode>, ParseError> {
        Ok(self.nodes(xpath)?.pop())
    }

    /// `$crawler->filterXPath($xpath)->count()`.
    pub fn count(&self, xpath: &str) -> Result<usize, ParseError> {
        Ok(self.nodes(xpath)?.len())
    }

    /// `$crawler->filterXPath($xpath)->attr($name)`.
    pub fn attr(&self, xpath: &str, name: &str) -> Result<Option<String>, ParseError> {
        Ok(self.first(xpath)?.and_then(|n| n.node_attr(name)))
    }

    /// `$crawler->filterXPath($xpath)->text()` with PHP whitespace
    /// normalization.
    pub fn text(&self, xpath: &str) -> Result<Option<String>, ParseError> {
        Ok(self.first(xpath)?.map(|n| n.node_text()))
    }

    /// `Parser::textOrNull($crawler->filterXPath($xpath))`.
    pub fn text_or_null(&self, xpath: &str) -> Result<Option<String>, ParseError> {
        self.text(xpath)
    }

    /// XPath `string($xpath)` against the document root.
    ///
    /// Unlike [`HtmlDoc::nodes`] the expression is **not** relativized: pass
    /// `string(//span[@itemprop='ratingValue'])` or a plain relative path.
    pub fn string_value(&self, xpath: &str) -> Result<String, ParseError> {
        string_value_ctx(&self.document, &self.root, xpath)
    }

    /// `$crawler->filterXPath($xpath)->html()` (inner HTML of the first node).
    pub fn html(&self, xpath: &str) -> Result<Option<String>, ParseError> {
        Ok(self.first(xpath)?.map(|n| n.node_html()))
    }

    /// `$crawler->filterXPath($xpath)->outerHtml()`.
    pub fn outer_html(&self, xpath: &str) -> Result<Option<String>, ParseError> {
        Ok(self.first(xpath)?.map(|n| n.outer_html()))
    }

    /// `$crawler->filter($selector)` (CSS selector -> XPath translation).
    pub fn css_nodes(&self, selector: &str) -> Result<Vec<HtmlNode>, ParseError> {
        let xpath = css_to_xpath(selector)?;
        let nodes = eval_nodes(&self.document, std::slice::from_ref(&self.root), &xpath)?;
        Ok(wrap_nodes(&self.document, nodes))
    }

    /// `$crawler->filterXPath($xpath)->each($closure)`.
    pub fn each<R>(
        &self,
        xpath: &str,
        mut f: impl FnMut(&HtmlNode, usize) -> R,
    ) -> Result<Vec<R>, ParseError> {
        let nodes = self.nodes(xpath)?;
        Ok(nodes.iter().enumerate().map(|(i, n)| f(n, i)).collect())
    }

    /// `Parser::removeChildNodes($crawler->filterXPath($xpath))`.
    pub fn remove_child_nodes(&self, xpath: &str) -> Result<(), ParseError> {
        match self.first(xpath)? {
            Some(node) => node.remove_child_nodes(),
            None => Ok(()),
        }
    }
}

impl HtmlNode {
    /// Wrap a raw libxml2 node (used by sibling modules/tests).
    pub fn new(document: Document, node: Node) -> Self {
        HtmlNode { document, node }
    }

    /// The underlying libxml2 node.
    pub fn node(&self) -> &Node {
        &self.node
    }

    /// The owning libxml2 document.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// Value of this node's own attribute (`Crawler::attr()` on the selection).
    pub fn node_attr(&self, name: &str) -> Option<String> {
        self.node.get_property(name)
    }

    /// `$crawler->text()`: string value of the node with DomCrawler whitespace
    /// normalization.
    pub fn node_text(&self) -> String {
        normalize_whitespace(&self.node.get_content())
    }

    /// `$crawler->text(null, false)`: unnormalized string value.
    pub fn node_text_raw(&self) -> String {
        self.node.get_content()
    }

    /// `$crawler->nodeName()` (lowercase for documents parsed as HTML).
    pub fn node_name(&self) -> String {
        self.node.get_name()
    }

    /// `$crawler->html()`: inner HTML of this node.
    ///
    /// Uses `htmlNodeDump` so void elements serialize like PHP's
    /// `DOMDocument::saveHTML` (`<br>`, not `<br/>`).
    pub fn node_html(&self) -> String {
        let mut out = String::new();
        for child in self.node.get_child_nodes() {
            out.push_str(&html_node_to_string(&self.document, &child));
        }
        out
    }

    /// `$crawler->outerHtml()`.
    pub fn outer_html(&self) -> String {
        html_node_to_string(&self.document, &self.node)
    }

    /// `$crawler->filterXPath($xpath)` relative to this node.
    pub fn nodes(&self, xpath: &str) -> Result<Vec<HtmlNode>, ParseError> {
        let nodes = eval_nodes(&self.document, std::slice::from_ref(&self.node), xpath)?;
        Ok(wrap_nodes(&self.document, nodes))
    }

    /// `$crawler->filterXPath($xpath)->first()`.
    pub fn first(&self, xpath: &str) -> Result<Option<HtmlNode>, ParseError> {
        Ok(self.nodes(xpath)?.into_iter().next())
    }

    /// `$crawler->filterXPath($xpath)->last()`.
    pub fn last(&self, xpath: &str) -> Result<Option<HtmlNode>, ParseError> {
        Ok(self.nodes(xpath)?.pop())
    }

    /// `$crawler->filterXPath($xpath)->count()`.
    pub fn count(&self, xpath: &str) -> Result<usize, ParseError> {
        Ok(self.nodes(xpath)?.len())
    }

    /// `$crawler->filterXPath($xpath)->attr($name)`.
    pub fn attr(&self, xpath: &str, name: &str) -> Result<Option<String>, ParseError> {
        Ok(self.first(xpath)?.and_then(|n| n.node_attr(name)))
    }

    /// `$crawler->filterXPath($xpath)->text()`.
    pub fn text(&self, xpath: &str) -> Result<Option<String>, ParseError> {
        Ok(self.first(xpath)?.map(|n| n.node_text()))
    }

    /// `Parser::textOrNull($crawler->filterXPath($xpath))`.
    pub fn text_or_null(&self, xpath: &str) -> Result<Option<String>, ParseError> {
        self.text(xpath)
    }

    /// XPath `string($xpath)` with this node as context.
    pub fn string_value(&self, xpath: &str) -> Result<String, ParseError> {
        string_value_ctx(&self.document, &self.node, xpath)
    }

    /// `$crawler->filterXPath($xpath)->html()`.
    pub fn html(&self, xpath: &str) -> Result<Option<String>, ParseError> {
        Ok(self.first(xpath)?.map(|n| n.node_html()))
    }

    /// `$crawler->filter($selector)`.
    pub fn css_nodes(&self, selector: &str) -> Result<Vec<HtmlNode>, ParseError> {
        let xpath = css_to_xpath(selector)?;
        let nodes = eval_nodes(&self.document, std::slice::from_ref(&self.node), &xpath)?;
        Ok(wrap_nodes(&self.document, nodes))
    }

    /// `$crawler->filterXPath($xpath)->each($closure)`.
    pub fn each<R>(
        &self,
        xpath: &str,
        mut f: impl FnMut(&HtmlNode, usize) -> R,
    ) -> Result<Vec<R>, ParseError> {
        let nodes = self.nodes(xpath)?;
        Ok(nodes.iter().enumerate().map(|(i, n)| f(n, i)).collect())
    }

    /// `$crawler->children()`: element children only (DomCrawler skips text
    /// nodes here).
    pub fn children(&self) -> Vec<HtmlNode> {
        wrap_nodes(&self.document, self.node.get_child_elements())
    }

    /// `$crawler->ancestors()`: element ancestors, nearest first.
    pub fn ancestors(&self) -> Vec<HtmlNode> {
        let mut out = Vec::new();
        let mut current = self.node.get_parent();
        while let Some(node) = current {
            if node.is_element_node() {
                out.push(HtmlNode {
                    document: self.document.clone(),
                    node: node.clone(),
                });
            }
            current = node.get_parent();
        }
        out
    }

    /// `$crawler->nextAll()`: following element siblings.
    pub fn next_all(&self) -> Vec<HtmlNode> {
        let mut out = Vec::new();
        let mut current = self.node.get_next_element_sibling();
        while let Some(node) = current {
            let next = node.get_next_element_sibling();
            out.push(HtmlNode {
                document: self.document.clone(),
                node,
            });
            current = next;
        }
        out
    }

    /// `$crawler->eq($index)`.
    pub fn eq(&self, _index: usize) -> Option<HtmlNode> {
        // `eq()` on a single-node wrapper: only index 0 is meaningful.
        if _index == 0 {
            Some(self.clone())
        } else {
            None
        }
    }

    /// Port of `Parser::removeChildNodes()`.
    ///
    /// Removes every element child that is not one of `p, i, b, br, strong, u`
    /// so that the remaining text can be read with `text()`.
    pub fn remove_child_nodes(&self) -> Result<(), ParseError> {
        const ALLOWED_NODES: [&str; 6] = ["p", "i", "b", "br", "strong", "u"];
        for child in self.node.get_child_elements() {
            let name = child.get_name().to_ascii_lowercase();
            if !ALLOWED_NODES.contains(&name.as_str()) {
                // Same as PHP `$node->parentNode->removeChild($node)`; the node
                // stays owned by the document (detached, not freed).
                unsafe {
                    libxml::bindings::xmlUnlinkNode(child.node_ptr());
                }
            }
        }
        Ok(())
    }

    /// `$crawler->innerText()`: direct text children only, normalized.
    pub fn inner_text(&self) -> String {
        for child in self.node.get_child_nodes() {
            if child.is_text_node() {
                let value = child.get_content();
                if !value.trim().is_empty() {
                    return normalize_whitespace(&value);
                }
            }
        }
        String::new()
    }
}

impl PartialEq for HtmlNode {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl Eq for HtmlNode {}

// ---------------------------------------------------------------------------
// HTML parsing helpers (Media.php / Parser.php string helpers)
// ---------------------------------------------------------------------------

/// `Parser::parseImageQuality()`.
pub fn parse_image_quality(image_url: &str) -> String {
    // adding `v` prefix returns a very small thumbnail, as opposed to adding `l`
    let image_url = image_url
        .replace("v.jpg", ".jpg")
        .replace("t.jpg", ".jpg")
        .replace("l.jpg", ".jpg");
    image_quality_re().replace_all(&image_url, "").to_string()
}

/// `Parser::parseImageThumbToHQ()`.
pub fn parse_image_thumb_to_hq(image_url: &str) -> String {
    image_url.replace("thumbs/", "").replace("_thumb", "")
}

/// `Media::youtubeIdFromUrl()`.
pub fn youtube_id_from_url(url: Option<&str>) -> Option<String> {
    let url = url?;
    youtube_re().captures(url).map(|caps| caps[1].to_string())
}

/// `Media::generateYoutubeUrlFromId()`.
pub fn generate_youtube_url_from_id(id: Option<&str>) -> Option<String> {
    id.map(|id| format!("https://www.youtube.com/watch?v={id}"))
}

/// `Media::generateYoutubeImageResource()` + JMS serialization.
pub fn youtube_image_resource(id: Option<&str>) -> Value {
    match id {
        None => json!({
            "image_url": null,
            "small_image_url": null,
            "medium_image_url": null,
            "large_image_url": null,
            "maximum_image_url": null,
        }),
        Some(id) => json!({
            "image_url": format!("https://img.youtube.com/vi/{id}/default.jpg"),
            "small_image_url": format!("https://img.youtube.com/vi/{id}/sddefault.jpg"),
            "medium_image_url": format!("https://img.youtube.com/vi/{id}/mqdefault.jpg"),
            "large_image_url": format!("https://img.youtube.com/vi/{id}/hqdefault.jpg"),
            "maximum_image_url": format!("https://img.youtube.com/vi/{id}/maxresdefault.jpg"),
        }),
    }
}

// ---------------------------------------------------------------------------
// low-level plumbing
// ---------------------------------------------------------------------------

/// Serialize a node with libxml2's HTML serializer (`htmlNodeDumpFormatOutput`
/// with `format = 0`), mirroring PHP `DOMDocument::saveHTML()`: void elements
/// are `<br>`, entities are re-encoded (`&amp;`) and no indentation is added.
fn html_node_to_string(document: &Document, node: &Node) -> String {
    use std::ffi::CStr;
    use std::ptr;
    unsafe {
        let buf = libxml::bindings::xmlBufferCreate();
        if buf.is_null() {
            return String::new();
        }
        let out = libxml::bindings::xmlOutputBufferCreateBuffer(buf, ptr::null_mut());
        if out.is_null() {
            libxml::bindings::xmlBufferFree(buf);
            return String::new();
        }
        libxml::bindings::htmlNodeDumpFormatOutput(
            out,
            document.doc_ptr(),
            node.node_ptr(),
            ptr::null(),
            0,
        );
        libxml::bindings::xmlOutputBufferFlush(out);
        libxml::bindings::xmlOutputBufferClose(out);
        let content = libxml::bindings::xmlBufferContent(buf);
        let result = if content.is_null() {
            String::new()
        } else {
            CStr::from_ptr(content as *const std::os::raw::c_char)
                .to_string_lossy()
                .into_owned()
        };
        libxml::bindings::xmlBufferFree(buf);
        result
    }
}

fn wrap_nodes(document: &Document, nodes: Vec<Node>) -> Vec<HtmlNode> {
    nodes
        .into_iter()
        .map(|node| HtmlNode {
            document: document.clone(),
            node,
        })
        .collect()
}

fn eval_nodes(
    document: &Document,
    contexts: &[Node],
    xpath: &str,
) -> Result<Vec<Node>, ParseError> {
    let relativized = relativize(xpath);
    let mut ctx =
        Context::new(document).map_err(|_| ParseError::InvalidXPath(xpath.to_string()))?;
    let mut out = Vec::new();
    for node in contexts {
        let found = ctx
            .findnodes(&relativized, Some(node))
            .map_err(|_| ParseError::InvalidXPath(xpath.to_string()))?;
        out.extend(found);
    }
    Ok(out)
}

fn string_value_ctx(document: &Document, node: &Node, xpath: &str) -> Result<String, ParseError> {
    let mut ctx =
        Context::new(document).map_err(|_| ParseError::InvalidXPath(xpath.to_string()))?;
    ctx.findvalue(xpath, Some(node))
        .map_err(|_| ParseError::InvalidXPath(xpath.to_string()))
}

/// Port of `Crawler::normalizeWhitespace()`.
pub fn normalize_whitespace(string: &str) -> String {
    // PHP: trim(preg_replace("/(?:[ \n\r\t\x0C]{2,}+|[\n\r\t\x0C])/", ' ', $s),
    //                       " \n\r\t\x0C")
    // Every maximal run of whitespace-in-class collapses to a single space,
    // which makes the final trim equivalent to trimming spaces.
    let mut out = String::with_capacity(string.len());
    let mut in_run = false;
    for c in string.chars() {
        if is_dom_space(c) {
            if !in_run {
                out.push(' ');
                in_run = true;
            }
        } else {
            out.push(c);
            in_run = false;
        }
    }
    out.trim_matches(' ').to_string()
}

fn is_dom_space(c: char) -> bool {
    matches!(c, ' ' | '\n' | '\r' | '\t' | '\u{0C}')
}

fn decode_html(bytes: &[u8]) -> String {
    let mut charset: String = if std::str::from_utf8(bytes).is_ok() {
        "utf-8".to_string()
    } else {
        "iso-8859-1".to_string()
    };

    if let Some(caps) = meta_charset_re().captures(bytes) {
        // PHP only accepts the captured token when it isn't the literal
        // 'charset=' (self-referential meta tags).
        if &caps[2] != b"charset=" {
            if let Ok(label) = std::str::from_utf8(&caps[2]) {
                charset = label.to_string();
            }
        }
    }

    let decoded = match encoding_rs::Encoding::for_label(charset.as_bytes()) {
        Some(encoding) if encoding != encoding_rs::UTF_8 => {
            let (decoded, _actual, _had_errors) = encoding.decode(bytes);
            decoded.into_owned()
        }
        _ => String::from_utf8_lossy(bytes).into_owned(),
    };

    // libxml2's HTML parser honours the document's `<meta charset>` over the
    // encoding passed to `htmlReadMemory`. The content is UTF-8 now, so rewrite
    // the declaration to match. (Symfony gets the same result by converting
    // every non-ASCII byte to a numeric entity before `loadHTML`.)
    meta_charset_str_re()
        .replace_all(&decoded, "${1}utf-8")
        .to_string()
}

fn meta_charset_re() -> &'static BytesRegex {
    static RE: OnceLock<BytesRegex> = OnceLock::new();
    RE.get_or_init(|| {
        BytesRegex::new(r#"(?i)(<meta[^>]+charset *= *["']?)([a-zA-Z\-0-9_:.]+)"#)
            .expect("valid regex")
    })
}

fn meta_charset_str_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?i)(<meta[^>]+charset *= *["']?)[a-zA-Z\-0-9_:.]+"#).expect("valid regex")
    })
}

fn image_quality_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"/r/\d+x\d+").expect("valid regex"))
}

fn youtube_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"^(?:https?://)?(?:www\.)?(?:m\.)?(?:youtu\.be/|youtube\.com/(?:(?:watch)?\?(?:.*&)?v(?:i)?=|(?:embed|v|vi|user|shorts)/))([^?&"'>]{11})"#,
        )
        .expect("valid regex")
    })
}

/// Port of `Crawler::relativize()` (private in DomCrawler).
///
/// Rewrites an XPath so that it is relative to the nodes of the current
/// crawler: leading `//` becomes `descendant-or-self::`, `.//` likewise,
/// `./`/`child::` become `self::`, absolute/unsupported expressions become a
/// never-matching expression and unions are handled element by element.
pub fn relativize(xpath: &str) -> String {
    const NON_MATCHING_EXPRESSION: &str = "a[name() = \"b\"]";
    const SCAN_SET: &[u8] = b"\"'[]|";
    let bytes = xpath.as_bytes();
    let len = bytes.len();
    let mut expressions: Vec<String> = Vec::new();
    let mut opened_brackets: i64 = 0;
    let mut start_position = strspn(bytes, 0, SPACE_CHARS);

    // `for ($i = $startPosition; $i <= $xpathLen; ++$i)`, with the `continue 2`
    // cases modelled as an explicit `i += 1` before `continue`.
    let mut i = start_position;
    while i <= len {
        i += strcspn(bytes, i, SCAN_SET);

        if i < len {
            match bytes[i] {
                quote @ (b'"' | b'\'') => match memchr(bytes, quote, i + 1) {
                    Some(pos) => {
                        i = pos + 1;
                        continue;
                    }
                    None => return xpath.to_string(),
                },
                b'[' => {
                    opened_brackets += 1;
                    i += 1;
                    continue;
                }
                b']' => {
                    opened_brackets -= 1;
                    i += 1;
                    continue;
                }
                _ => {}
            }
        }
        if opened_brackets != 0 {
            i += 1;
            continue;
        }

        let start = start_position;
        let (parenthesis, expression) = if start < len && bytes[start] == b'(' {
            // Preserve a leading `(` (+ following spaces) and process the
            // expression inside it, so `(//a | //b)` stays one union.
            let j = 1 + strspn(bytes, start + 1, b"( \t\n\r\x00\x0B");
            let parenthesis = xpath[start..start + j].to_string();
            let expression = xpath[start + j..i].trim_end().to_string();
            (parenthesis, expression)
        } else {
            (String::new(), xpath[start..i].trim_end().to_string())
        };

        let mapped = map_expression(&expression, NON_MATCHING_EXPRESSION);
        expressions.push(format!("{parenthesis}{mapped}"));

        if i == len {
            return expressions.join(" | ");
        }

        i += strspn(bytes, i + 1, SPACE_CHARS) + 1;
        start_position = i;
    }

    xpath.to_string()
}

fn map_expression(expression: &str, non_matching: &str) -> String {
    let expression = match expression.strip_prefix("self::*/") {
        Some(rest) => format!("./{rest}"),
        None => expression.to_string(),
    };
    let expression = expression.as_str();

    if expression.is_empty() {
        return non_matching.to_string();
    }
    if let Some(rest) = expression.strip_prefix("//") {
        return format!("descendant-or-self::{rest}");
    }
    if let Some(rest) = expression.strip_prefix(".//") {
        return format!("descendant-or-self::{rest}");
    }
    if let Some(rest) = expression.strip_prefix("./") {
        return format!("self::{rest}");
    }
    if let Some(rest) = expression.strip_prefix("child::") {
        return format!("self::{rest}");
    }
    if expression.starts_with('/')
        || expression.starts_with('.')
        || expression.starts_with("self::")
    {
        return non_matching.to_string();
    }
    if let Some(rest) = expression.strip_prefix("descendant::") {
        return format!("descendant-or-self::{rest}");
    }
    if ancestor_axis_re().is_match(expression) {
        return non_matching.to_string();
    }
    if !expression.starts_with("descendant-or-self::") {
        return format!("self::{expression}");
    }
    expression.to_string()
}

fn strspn(bytes: &[u8], from: usize, set: &[u8]) -> usize {
    let mut count = 0;
    while from + count < bytes.len() && set.contains(&bytes[from + count]) {
        count += 1;
    }
    count
}

fn strcspn(bytes: &[u8], from: usize, set: &[u8]) -> usize {
    let mut count = 0;
    while from + count < bytes.len() && !set.contains(&bytes[from + count]) {
        count += 1;
    }
    count
}

fn memchr(bytes: &[u8], needle: u8, from: usize) -> Option<usize> {
    bytes[from..]
        .iter()
        .position(|b| *b == needle)
        .map(|p| p + from)
}

const SPACE_CHARS: &[u8] = b" \t\n\r\x00\x0B";

fn ancestor_axis_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^(ancestor|ancestor-or-self|attribute|following|following-sibling|namespace|parent|preceding|preceding-sibling)::",
        )
        .expect("valid regex")
    })
}

// ---------------------------------------------------------------------------
// CSS -> XPath (port of the Symfony CssSelector subset used by jikan-php)
// ---------------------------------------------------------------------------

/// Translate a CSS selector with the same rules as
/// `Symfony\Component\CssSelector\CssSelectorConverter(true)->toXPath()`.
pub fn css_to_xpath(selector: &str) -> Result<String, ParseError> {
    let selector = selector.trim();
    if selector.is_empty() {
        return Err(ParseError::InvalidSelector("empty selector".into()));
    }
    let groups = split_selector_list(selector)?;
    let mut xpaths = Vec::with_capacity(groups.len());
    for group in groups {
        xpaths.push(selector_to_xpath(group.trim())?);
    }
    Ok(xpaths.join(" | "))
}

fn invalid(selector: &str, reason: &str) -> ParseError {
    ParseError::InvalidSelector(format!("{selector:?}: {reason}"))
}

fn split_selector_list(selector: &str) -> Result<Vec<&str>, ParseError> {
    let bytes = selector.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0;
    let mut i = 0;
    let mut bracket_depth = 0i32;
    let mut quote: Option<u8> = None;
    while i < bytes.len() {
        let b = bytes[i];
        match quote {
            Some(q) => {
                if b == q {
                    quote = None;
                }
            }
            None => match b {
                b'"' | b'\'' => quote = Some(b),
                b'[' => bracket_depth += 1,
                b']' => bracket_depth -= 1,
                b',' if bracket_depth == 0 => {
                    parts.push(selector[start..i].trim());
                    start = i + 1;
                }
                _ => {}
            },
        }
        i += 1;
    }
    if quote.is_some() || bracket_depth != 0 {
        return Err(invalid(selector, "unbalanced quotes or brackets"));
    }
    parts.push(selector[start..].trim());
    if parts.iter().any(|p| p.is_empty()) {
        return Err(invalid(selector, "empty selector in group"));
    }
    Ok(parts)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Combinator {
    Descendant,
    Child,
    Adjacent,
    GeneralSibling,
}

#[derive(Debug, Default)]
struct Compound {
    element: Option<String>,
    conditions: Vec<String>,
}

fn selector_to_xpath(selector: &str) -> Result<String, ParseError> {
    if selector.is_empty() {
        return Err(ParseError::InvalidSelector("empty selector".into()));
    }
    let bytes = selector.as_bytes();
    let mut i = 0;
    let mut steps: Vec<(Option<Combinator>, Compound)> = Vec::new();
    let mut pending: Option<Combinator> = None;

    loop {
        let had_space = skip_space(bytes, &mut i);
        if i >= bytes.len() {
            break;
        }
        let b = bytes[i];
        if matches!(b, b'>' | b'+' | b'~') {
            if pending.is_some() {
                return Err(invalid(selector, "consecutive combinators"));
            }
            pending = Some(match b {
                b'>' => Combinator::Child,
                b'+' => Combinator::Adjacent,
                _ => Combinator::GeneralSibling,
            });
            i += 1;
            skip_space(bytes, &mut i);
        } else if had_space && !steps.is_empty() && pending.is_none() {
            pending = Some(Combinator::Descendant);
        }

        if i >= bytes.len() {
            return Err(invalid(selector, "dangling combinator"));
        }

        let compound = parse_compound(selector, bytes, &mut i)?;
        let combinator = if steps.is_empty() {
            if pending == Some(Combinator::Descendant) {
                // A leading whitespace is just trimmed.
                pending = None;
            }
            pending.take()
        } else {
            pending.take().or(Some(Combinator::Descendant))
        };
        steps.push((combinator, compound));
    }

    if steps.is_empty() {
        return Err(invalid(selector, "no selector"));
    }

    let mut out = String::from("descendant-or-self::");
    out.push_str(&compound_to_xpath(&steps[0].1));
    for (combinator, compound) in &steps[1..] {
        match combinator {
            Some(Combinator::Descendant) | None => {
                out.push_str("/descendant-or-self::*/");
                out.push_str(&compound_to_xpath(compound));
            }
            Some(Combinator::Child) => {
                out.push('/');
                out.push_str(&compound_to_xpath(compound));
            }
            // `addNameTest()` + `position() = 1`, exactly like Symfony's
            // CombinationExtension::translateDirectAdjacent().
            Some(Combinator::Adjacent) => {
                let mut renamed = Compound {
                    element: Some("*".to_string()),
                    conditions: compound.conditions.clone(),
                };
                if let Some(element) = &compound.element {
                    if element != "*" {
                        renamed
                            .conditions
                            .push(format!("name() = {}", xpath_literal(element)));
                    }
                }
                let condition = combine_conditions(&renamed.conditions);
                let condition = if condition.is_empty() {
                    "position() = 1".to_string()
                } else {
                    format!("({condition}) and (position() = 1)")
                };
                out.push_str(&format!("/following-sibling::*[{condition}]"));
            }
            Some(Combinator::GeneralSibling) => {
                out.push_str("/following-sibling::");
                out.push_str(&compound_to_xpath(compound));
            }
        }
    }
    Ok(out)
}

fn skip_space(bytes: &[u8], i: &mut usize) -> bool {
    let start = *i;
    while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
        *i += 1;
    }
    *i > start
}

fn parse_compound(selector: &str, bytes: &[u8], i: &mut usize) -> Result<Compound, ParseError> {
    let mut compound = Compound::default();

    if *i < bytes.len() && bytes[*i] == b'*' {
        compound.element = Some("*".to_string());
        *i += 1;
    } else if *i < bytes.len() && is_ident_start(bytes[*i]) {
        let name = read_ident(selector, bytes, i)?;
        // HTML documents: CssSelector lowercases element names.
        compound.element = Some(name.to_ascii_lowercase());
    }

    loop {
        if *i >= bytes.len() {
            break;
        }
        match bytes[*i] {
            b'.' => {
                *i += 1;
                let name = read_ident(selector, bytes, i)?;
                compound.conditions.push(class_condition(&name));
            }
            b'#' => {
                *i += 1;
                let name = read_ident(selector, bytes, i)?;
                compound
                    .conditions
                    .push(format!("@id = {}", xpath_literal(&name)));
            }
            b'[' => {
                compound
                    .conditions
                    .push(parse_attribute(selector, bytes, i)?);
            }
            b':' => {
                compound
                    .conditions
                    .push(parse_pseudo(selector, bytes, i, &compound)?);
            }
            _ => break,
        }
    }
    Ok(compound)
}

fn parse_attribute(selector: &str, bytes: &[u8], i: &mut usize) -> Result<String, ParseError> {
    debug_assert_eq!(bytes[*i], b'[');
    *i += 1;
    skip_space(bytes, i);
    let name = read_ident(selector, bytes, i)?;
    skip_space(bytes, i);
    if *i >= bytes.len() {
        return Err(invalid(selector, "unterminated attribute selector"));
    }
    if bytes[*i] == b']' {
        *i += 1;
        return Ok(format!("@{name}"));
    }

    let op = if bytes[*i] == b'=' {
        *i += 1;
        "="
    } else if matches!(bytes[*i], b'^' | b'$' | b'*' | b'~' | b'|' | b'!')
        && *i + 1 < bytes.len()
        && bytes[*i + 1] == b'='
    {
        let op = match bytes[*i] {
            b'^' => "^=",
            b'$' => "$=",
            b'*' => "*=",
            b'~' => "~=",
            b'|' => "|=",
            _ => "!=",
        };
        *i += 2;
        op
    } else {
        return Err(invalid(selector, "unsupported attribute operator"));
    };
    skip_space(bytes, i);

    let value = if *i < bytes.len() && (bytes[*i] == b'"' || bytes[*i] == b'\'') {
        let quote = bytes[*i];
        *i += 1;
        let start = *i;
        while *i < bytes.len() && bytes[*i] != quote {
            *i += 1;
        }
        if *i >= bytes.len() {
            return Err(invalid(selector, "unterminated attribute value"));
        }
        let value = selector[start..*i].to_string();
        *i += 1;
        value
    } else {
        // Unquoted values may be identifiers or numbers (`[attr=123]`).
        let start = *i;
        while *i < bytes.len()
            && !matches!(bytes[*i], b']' | b' ' | b'\t' | b'\n' | b'\r' | b'\x0C')
        {
            *i += 1;
        }
        if start == *i {
            return Err(invalid(selector, "expected attribute value"));
        }
        selector[start..*i].to_string()
    };
    skip_space(bytes, i);
    if *i >= bytes.len() || bytes[*i] != b']' {
        return Err(invalid(selector, "unterminated attribute selector"));
    }
    *i += 1;

    let attr = format!("@{name}");
    let condition = match op {
        "=" => format!("{attr} = {}", xpath_literal(&value)),
        "!=" => {
            if value.is_empty() {
                format!("{attr} != {}", xpath_literal(&value))
            } else {
                format!("not({attr}) or {attr} != {}", xpath_literal(&value))
            }
        }
        "~=" => {
            if value.is_empty() {
                "0".to_string()
            } else {
                format!(
                    "{attr} and contains(concat(' ', normalize-space({attr}), ' '), {})",
                    xpath_literal(&format!(" {value} "))
                )
            }
        }
        "|=" => format!(
            "{attr} and ({attr} = {} or starts-with({attr}, {}))",
            xpath_literal(&value),
            xpath_literal(&format!("{value}-"))
        ),
        "^=" => {
            if value.is_empty() {
                "0".to_string()
            } else {
                format!("{attr} and starts-with({attr}, {})", xpath_literal(&value))
            }
        }
        "$=" => {
            if value.is_empty() {
                "0".to_string()
            } else {
                let n = value.len().saturating_sub(1);
                format!(
                    "{attr} and substring({attr}, string-length({attr})-{n}) = {}",
                    xpath_literal(&value)
                )
            }
        }
        "*=" => {
            if value.is_empty() {
                "0".to_string()
            } else {
                format!("{attr} and contains({attr}, {})", xpath_literal(&value))
            }
        }
        _ => return Err(invalid(selector, "unsupported attribute operator")),
    };
    Ok(condition)
}

fn parse_pseudo(
    selector: &str,
    bytes: &[u8],
    i: &mut usize,
    outer: &Compound,
) -> Result<String, ParseError> {
    debug_assert_eq!(bytes[*i], b':');
    if selector[*i..].starts_with("::") {
        return Err(invalid(selector, "pseudo-elements are not supported"));
    }
    if selector[*i..].starts_with(":not(") {
        let inner_start = *i + 5;
        let mut depth = 1;
        let mut j = inner_start;
        while j < bytes.len() {
            match bytes[j] {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            j += 1;
        }
        if j >= bytes.len() {
            return Err(invalid(selector, "unterminated :not()"));
        }
        let inner = &selector[inner_start..j];
        let inner_compound = parse_full_compound(selector, inner)?;
        let mut inner_conditions = inner_compound.conditions.clone();
        // `XPathExpr::addNameTest()`: element becomes `*` and the name is
        // appended as a condition (after any existing ones).
        if let Some(element) = &inner_compound.element {
            if element != "*" {
                inner_conditions.push(format!("name() = {}", xpath_literal(element)));
            }
        }
        let condition = combine_conditions(&inner_conditions);
        *i = j + 1;
        return Ok(if condition.is_empty() {
            "0".to_string()
        } else {
            format!("not({condition})")
        });
    }
    if selector[*i..].starts_with(":nth-child(") {
        return Err(invalid(
            selector,
            ":nth-child() is not supported by the Kuukan CSS translator",
        ));
    }
    let _ = outer;
    Err(invalid(
        selector,
        "unsupported pseudo-class (only :not(simple) is translated)",
    ))
}

fn parse_full_compound(selector: &str, text: &str) -> Result<Compound, ParseError> {
    // `parse_compound` slices the text it is given, so `selector` (used only
    // for error messages) and `text` must not be mixed up here.
    let bytes = text.as_bytes();
    let mut i = 0;
    skip_space(bytes, &mut i);
    if i >= bytes.len() {
        return Err(invalid(selector, "empty :not() argument"));
    }
    let compound = parse_compound(text, bytes, &mut i)?;
    skip_space(bytes, &mut i);
    if i != bytes.len() {
        return Err(invalid(
            selector,
            ":not() argument is not a simple selector",
        ));
    }
    Ok(compound)
}

fn compound_to_xpath(compound: &Compound) -> String {
    let element = compound.element.as_deref().unwrap_or("*");
    let condition = combine_conditions(&compound.conditions);
    if condition.is_empty() {
        element.to_string()
    } else {
        format!("{element}[{condition}]")
    }
}

fn combine_conditions(conditions: &[String]) -> String {
    let mut out = String::new();
    for condition in conditions {
        out = if out.is_empty() {
            condition.clone()
        } else {
            format!("({out}) and ({condition})")
        };
    }
    out
}

fn class_condition(name: &str) -> String {
    format!(
        "@class and contains(concat(' ', normalize-space(@class), ' '), {})",
        xpath_literal(&format!(" {name} "))
    )
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b >= 0x80
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b >= 0x80
}

fn read_ident(selector: &str, bytes: &[u8], i: &mut usize) -> Result<String, ParseError> {
    let start = *i;
    if start >= bytes.len() || !is_ident_start(bytes[start]) {
        return Err(invalid(selector, "expected identifier"));
    }
    while *i < bytes.len() && is_ident_char(bytes[*i]) {
        *i += 1;
    }
    Ok(selector[start..*i].to_string())
}

/// Port of `Translator::getXpathLiteral()`.
pub fn xpath_literal(s: &str) -> String {
    if !s.contains('\'') {
        return format!("'{s}'");
    }
    if !s.contains('"') {
        return format!("\"{s}\"");
    }
    let mut parts = Vec::new();
    let mut rest = s;
    loop {
        match rest.find('\'') {
            Some(pos) => {
                parts.push(format!("'{}'", &rest[..pos]));
                parts.push("\"'\"".to_string());
                rest = &rest[pos + 1..];
            }
            None => {
                parts.push(format!("'{rest}'"));
                break;
            }
        }
    }
    format!("concat({})", parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal stand-in for the recorded MAL page used during the port: the
    /// same nodes the assertions below exercise, generated inline so the test
    /// has no external fixtures.
    fn fixture_html() -> String {
        let mut pads = String::from("<div class=\"spaceit_pad\">Japanese: トライガン</div>");
        for i in 0..32 {
            pads.push_str(&format!("<div class=\"spaceit_pad\">field {i}</div>"));
        }
        pads.push_str("<div class=\"spaceit_pad\"><span class=\"score-label\">8.22</span></div>");
        format!(
            "<html><head><meta property=\"og:title\" content=\"Trigun\"/></head><body>\
             <h2>Alternative Titles</h2><div>Japanese: トライガン</div>{pads}\
             <span itemprop=\"ratingValue\">8.22</span></body></html>"
        )
    }

    #[test]
    fn parses_inline_markup() {
        let bytes = fixture_html().into_bytes();
        let doc = HtmlDoc::parse(&bytes).expect("html parses");

        // og:title
        let title = doc
            .attr("//meta[@property='og:title']", "content")
            .expect("valid xpath")
            .expect("og:title");
        assert_eq!(title, "Trigun");

        // ratingValue via XPath string(). The brief mentioned 8.23, but the
        // recorded fixture (and upstream AnimeParserTest) says 8.22.
        // Expected values captured from Symfony DomCrawler over the same file.
        assert_eq!(
            doc.text("//div[@class='spaceit_pad']").unwrap().as_deref(),
            Some("Japanese: トライガン")
        );
        assert_eq!(doc.count("//div[@class='spaceit_pad']").unwrap(), 34);
        assert_eq!(
            doc.text("//h2[contains(., 'Alternative Titles')]/following-sibling::div[1]")
                .unwrap()
                .as_deref(),
            Some("Japanese: トライガン")
        );
        assert_eq!(
            doc.html("//span[@itemprop='ratingValue']")
                .unwrap()
                .as_deref(),
            Some("8.22")
        );

        // Regression guard: parsing the same bytes must be deterministic (the
        // libxml encoding-argument path used to drop nodes ~10% of the time).
        for _ in 0..25 {
            let doc2 = HtmlDoc::parse(&bytes).unwrap();
            assert_eq!(
                doc2.string_value("string(//span[@itemprop='ratingValue'])")
                    .unwrap(),
                "8.22"
            );
        }
        let rating = doc
            .string_value("string(//span[@itemprop='ratingValue'])")
            .expect("valid xpath");
        assert_eq!(rating, "8.22");

        // CSS -> XPath for a `.spaceit_pad .score-label`-style selector
        let score = doc
            .css_nodes(".spaceit_pad .score-label")
            .expect("selector translates")
            .first()
            .map(|n| n.node_text())
            .expect("score span");
        assert_eq!(score, "8.22");
    }

    #[test]
    fn css_translation_matches_symfony() {
        let cases: &[(&str, &str)] = &[
            (
                "div.foo",
                "descendant-or-self::div[@class and contains(concat(' ', normalize-space(@class), ' '), ' foo ')]",
            ),
            (
                ".foo",
                "descendant-or-self::*[@class and contains(concat(' ', normalize-space(@class), ' '), ' foo ')]",
            ),
            ("#id", "descendant-or-self::*[@id = 'id']"),
            (
                "div a",
                "descendant-or-self::div/descendant-or-self::*/a",
            ),
            ("div > a", "descendant-or-self::div/a"),
            (
                "div, span",
                "descendant-or-self::div | descendant-or-self::span",
            ),
            ("[attr]", "descendant-or-self::*[@attr]"),
            (
                "[attr=\"v\"]",
                "descendant-or-self::*[@attr = 'v']",
            ),
            (
                "a[href^=\"https://x\"]",
                "descendant-or-self::a[@href and starts-with(@href, 'https://x')]",
            ),
            (
                "div.seasonal-anime.js-seasonal-anime",
                "descendant-or-self::div[(@class and contains(concat(' ', normalize-space(@class), ' '), ' seasonal-anime ')) and (@class and contains(concat(' ', normalize-space(@class), ' '), ' js-seasonal-anime '))]",
            ),
            (
                "div.navi-seasonal a.on",
                "descendant-or-self::div[@class and contains(concat(' ', normalize-space(@class), ' '), ' navi-seasonal ')]/descendant-or-self::*/a[@class and contains(concat(' ', normalize-space(@class), ' '), ' on ')]",
            ),
            (
                "div:not(.x)",
                "descendant-or-self::div[not(@class and contains(concat(' ', normalize-space(@class), ' '), ' x '))]",
            ),
            (
                "div:not(span)",
                "descendant-or-self::div[not(name() = 'span')]",
            ),
            (
                "span:not(div.x)",
                "descendant-or-self::span[not((@class and contains(concat(' ', normalize-space(@class), ' '), ' x ')) and (name() = 'div'))]",
            ),
            (
                "[attr$=\"v\"]",
                "descendant-or-self::*[@attr and substring(@attr, string-length(@attr)-0) = 'v']",
            ),
            (
                "[attr*=\"v\"]",
                "descendant-or-self::*[@attr and contains(@attr, 'v')]",
            ),
            (
                "[attr~=\"v\"]",
                "descendant-or-self::*[@attr and contains(concat(' ', normalize-space(@attr), ' '), ' v ')]",
            ),
            (
                "[attr|=\"v\"]",
                "descendant-or-self::*[@attr and (@attr = 'v' or starts-with(@attr, 'v-'))]",
            ),
            (
                "[attr!=\"v\"]",
                "descendant-or-self::*[not(@attr) or @attr != 'v']",
            ),
            ("[attr=123]", "descendant-or-self::*[@attr = '123']"),
            (
                "p + a",
                "descendant-or-self::p/following-sibling::*[(name() = 'a') and (position() = 1)]",
            ),
            ("p ~ a", "descendant-or-self::p/following-sibling::a"),
            (
                "p + a.foo",
                "descendant-or-self::p/following-sibling::*[((@class and contains(concat(' ', normalize-space(@class), ' '), ' foo ')) and (name() = 'a')) and (position() = 1)]",
            ),
            (
                ".a.b",
                "descendant-or-self::*[(@class and contains(concat(' ', normalize-space(@class), ' '), ' a ')) and (@class and contains(concat(' ', normalize-space(@class), ' '), ' b '))]",
            ),
            (
                "#a.b",
                "descendant-or-self::*[(@id = 'a') and (@class and contains(concat(' ', normalize-space(@class), ' '), ' b '))]",
            ),
            (
                "div > .x",
                "descendant-or-self::div/*[@class and contains(concat(' ', normalize-space(@class), ' '), ' x ')]",
            ),
            (
                "td, th, tr",
                "descendant-or-self::td | descendant-or-self::th | descendant-or-self::tr",
            ),
        ];
        for (selector, expected) in cases {
            assert_eq!(&css_to_xpath(selector).unwrap(), expected, "{selector}");
        }
        assert!(css_to_xpath("").is_err());
        assert!(css_to_xpath("div:hover").is_err());
        assert!(css_to_xpath("div[attr=]").is_err());
    }

    #[test]
    fn relativize_matches_domcrawler() {
        // Expected values captured from Symfony DomCrawler (private relativize)
        let cases: &[(&str, &str)] = &[
            (
                "//meta[@property=\"og:title\"]",
                "descendant-or-self::meta[@property=\"og:title\"]",
            ),
            ("//a/img", "descendant-or-self::a/img"),
            ("//div", "descendant-or-self::div"),
            ("descendant-or-self::div", "descendant-or-self::div"),
            ("descendant::div", "descendant-or-self::div"),
            ("./div", "self::div"),
            (".//div", "descendant-or-self::div"),
            ("child::div", "self::div"),
            ("self::div", "a[name() = \"b\"]"),
            ("div", "self::div"),
            ("table/tr", "self::table/tr"),
            (
                "(//a | //b)",
                "(descendant-or-self::a | descendant-or-self::b)",
            ),
            ("//a | //b", "descendant-or-self::a | descendant-or-self::b"),
            (
                "//a[1] | //b[2]",
                "descendant-or-self::a[1] | descendant-or-self::b[2]",
            ),
            (
                "//a[@x=\"|\"] | //b",
                "descendant-or-self::a[@x=\"|\"] | descendant-or-self::b",
            ),
            ("ancestor::div", "a[name() = \"b\"]"),
            (
                "//div[@class=\"a[b]\"]//span",
                "descendant-or-self::div[@class=\"a[b]\"]//span",
            ),
            ("self::*/div", "self::div"),
            ("", "a[name() = \"b\"]"),
            ("  ", "a[name() = \"b\"]"),
            (
                "//div[contains(text(), \"a|b\")]//span",
                "descendant-or-self::div[contains(text(), \"a|b\")]//span",
            ),
            ("//td[2]/span", "descendant-or-self::td[2]/span"),
        ];
        for (input, expected) in cases {
            assert_eq!(&relativize(input), expected, "{input}");
        }
    }

    #[test]
    fn normalize_whitespace_matches_domcrawler() {
        assert_eq!(normalize_whitespace("  a  b  "), "a b");
        assert_eq!(normalize_whitespace("a\nb"), "a b");
        assert_eq!(normalize_whitespace("a \n\t b"), "a b");
        assert_eq!(normalize_whitespace("a\u{a0}b"), "a\u{a0}b");
        assert_eq!(normalize_whitespace("\n\r\t\x0C"), "");
    }

    #[test]
    fn nodes_are_relative_to_context() {
        let doc = HtmlDoc::parse_str(
            "<html><body><div id='a'><span class='x'>1</span></div><div id='b'><span class='x'>2</span></div></body></html>",
        )
        .unwrap();
        assert_eq!(doc.count("//span[@class='x']").unwrap(), 2);
        let first_div = doc.first("//div[@id='a']").unwrap().unwrap();
        // DomCrawler relativizes `//span` to `descendant-or-self::span`
        assert_eq!(first_div.count("//span").unwrap(), 1);
        assert_eq!(first_div.text("//span").unwrap().as_deref(), Some("1"));
        // nextAll
        let siblings = first_div.next_all();
        assert_eq!(siblings.len(), 1);
        assert_eq!(siblings[0].node_attr("id").as_deref(), Some("b"));
        // ancestors
        let span = doc.first("//span").unwrap().unwrap();
        let names: Vec<String> = span.ancestors().iter().map(|n| n.node_name()).collect();
        assert_eq!(names, vec!["div", "body", "html"]);
    }

    #[test]
    fn remove_child_nodes_keeps_allowed() {
        let doc =
            HtmlDoc::parse_str("<div>text <b>bold</b> <span>drop</span> <i>keep</i> tail</div>")
                .unwrap();
        let div = doc.first("//div").unwrap().unwrap();
        div.remove_child_nodes().unwrap();
        let text = div.node_text();
        assert_eq!(text, "text bold keep tail");
        assert_eq!(div.count("//span").unwrap(), 0);
        assert_eq!(div.count("//b").unwrap(), 1);
    }

    #[test]
    fn html_and_outer_html() {
        let doc = HtmlDoc::parse_str("<div class='x'>a<br>b</div>").unwrap();
        assert_eq!(doc.html("//div").unwrap().as_deref(), Some("a<br>b"));
        let outer = doc.outer_html("//div").unwrap().unwrap();
        assert!(outer.contains("<div class=\"x\">"), "{outer}");
        assert!(outer.contains("a<br>b"), "{outer}");
    }

    /// Expected values captured from PHP 8.5 `DOMDocument::saveHTML` via
    /// Symfony DomCrawler.
    #[test]
    fn html_serialization_matches_php() {
        let doc = HtmlDoc::parse_str(
            "<html><body><div class=\"x\" data-a=\"b\">a<br>b<img src=\"x.png\" alt=\"y\">c &amp; d<!-- c --><span>s</span><input value=\"v\"></div></body></html>",
        )
        .unwrap();
        let inner = doc.html("//div").unwrap().unwrap();
        assert_eq!(
            inner,
            "a<br>b<img src=\"x.png\" alt=\"y\">c &amp; d<!-- c --><span>s</span><input value=\"v\">"
        );
        let outer = doc.outer_html("//div").unwrap().unwrap();
        assert_eq!(
            outer,
            "<div class=\"x\" data-a=\"b\">a<br>b<img src=\"x.png\" alt=\"y\">c &amp; d<!-- c --><span>s</span><input value=\"v\"></div>"
        );

        let doc = HtmlDoc::parse_str("<p>&lt;tag&gt; &quot;q&quot; &amp; &#39;s&#39;</p>").unwrap();
        assert_eq!(
            doc.text("//p").unwrap().as_deref(),
            Some("<tag> \"q\" & 's'")
        );
        assert_eq!(
            doc.html("//p").unwrap().as_deref(),
            Some("&lt;tag&gt; \"q\" &amp; 's'")
        );
    }

    #[test]
    fn attributes_and_text_missing_match_returns_none() {
        let doc = HtmlDoc::parse_str("<p>hello   world</p>").unwrap();
        assert_eq!(doc.text("//p").unwrap().as_deref(), Some("hello world"));
        assert!(doc.attr("//p", "id").unwrap().is_none());
        assert!(doc.first("//nope").unwrap().is_none());
        assert_eq!(doc.count("//nope").unwrap(), 0);
        assert_eq!(doc.string_value("string(//nope)").unwrap(), "");
    }

    #[test]
    fn empty_and_fragment_documents() {
        let doc = HtmlDoc::parse_str("<td>hi</td>").unwrap();
        assert_eq!(doc.text("//td").unwrap().as_deref(), Some("hi"));

        // An empty body has no root element, like an empty DomCrawler.
        assert!(matches!(
            HtmlDoc::parse_str("").unwrap_err(),
            ParseError::MissingRoot
        ));
    }

    #[test]
    fn invalid_xpath_is_error() {
        let doc = HtmlDoc::parse_str("<p>x</p>").unwrap();
        assert!(matches!(
            doc.nodes("//p[").unwrap_err(),
            ParseError::InvalidXPath(_)
        ));
    }

    #[test]
    fn charset_is_detected() {
        let latin1 = b"<html><head><meta http-equiv=\"Content-Type\" content=\"text/html; charset=iso-8859-1\"></head><body><p>caf\xe9</p></body></html>";
        let doc = HtmlDoc::parse(latin1).unwrap();
        assert_eq!(doc.text("//p").unwrap().as_deref(), Some("café"));
    }

    // Ported from test/JikanTest/Helper/MediaTest.php
    #[test]
    fn youtube_id_provider() {
        assert_eq!(
            youtube_id_from_url(Some(
                "https://www.youtube.com/embed/1yXa8MAmocQ?enablejsapi=1&wmode=opaque&autoplay=1"
            ))
            .as_deref(),
            Some("1yXa8MAmocQ")
        );
        assert_eq!(
            youtube_id_from_url(Some(
                "https://www.youtube.com/embed/yhNzL20gNX0/?enablejsapi=1&wmode=opaque&autoplay=1"
            ))
            .as_deref(),
            Some("yhNzL20gNX0")
        );
        assert_eq!(
            youtube_id_from_url(Some("https://youtu.be/dQw4w9WgXcQ")).as_deref(),
            Some("dQw4w9WgXcQ")
        );
        assert_eq!(youtube_id_from_url(None), None);
    }

    #[test]
    fn image_helpers() {
        assert_eq!(
            parse_image_quality("https://cdn.myanimelist.net/images/anime/1/1v.jpg"),
            "https://cdn.myanimelist.net/images/anime/1/1.jpg"
        );
        assert_eq!(
            parse_image_quality("https://cdn.myanimelist.net/r/100x140/images/anime/1/1.jpg"),
            "https://cdn.myanimelist.net/images/anime/1/1.jpg"
        );
        assert_eq!(
            parse_image_thumb_to_hq(
                "https://cdn.myanimelist.net/images/anime/1/thumbs/1_thumb.jpg"
            ),
            "https://cdn.myanimelist.net/images/anime/1/1.jpg"
        );
        assert_eq!(
            generate_youtube_url_from_id(Some("abc")),
            Some("https://www.youtube.com/watch?v=abc".to_string())
        );
        let images = youtube_image_resource(Some("abc"));
        assert_eq!(
            images["image_url"],
            "https://img.youtube.com/vi/abc/default.jpg"
        );
        assert_eq!(
            images["small_image_url"],
            "https://img.youtube.com/vi/abc/sddefault.jpg"
        );
        assert_eq!(
            images["medium_image_url"],
            "https://img.youtube.com/vi/abc/mqdefault.jpg"
        );
        assert_eq!(
            images["large_image_url"],
            "https://img.youtube.com/vi/abc/hqdefault.jpg"
        );
        assert_eq!(
            images["maximum_image_url"],
            "https://img.youtube.com/vi/abc/maxresdefault.jpg"
        );
        assert_eq!(youtube_image_resource(None)["image_url"], Value::Null);
    }
}
