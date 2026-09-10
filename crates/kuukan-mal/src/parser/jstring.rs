//! Port of `Jikan\Helper\JString` (jikan-php v4.0.12).
//!
//! The quirks matter: `<br>` variants become a literal backslash-`n` first,
//! then control characters are stripped, then `strip_tags()` runs, and only
//! afterwards is the literal `\n` turned into a real newline and the result
//! trimmed. Any pre-existing newline in the input is therefore removed.

/// Port of `JString::cleanse()`.
///
/// Converts MAL HTML snippets into the plain string Jikan serializes.
pub fn cleanse(string: &str) -> String {
    // convert any html before hand to new line
    // (PHP uses "\\n" / '\\n': a literal backslash followed by `n`)
    let mut out = string
        .replace("<br>", "\\n")
        .replace("<br />", "\\n")
        .replace("<br/>", "\\n")
        .replace("<br >", "\\n");

    // convert nbsp to space
    out = out.replace('\u{a0}', " ");

    // remove control characters (PCRE [[:cntrl:]] = 0x00-0x1F and 0x7F)
    out.retain(|c| !c.is_ascii_control());

    // strip any leftover tags
    out = strip_tags(&out);

    // remove any newlines at the end
    out = out.replace("\\n", "\n");

    // trim
    php_trim(&out)
}

/// Port of `JString::UTF8NbspTrim()`: `trim($string, chr(0xC2).chr(0xA0))`.
///
/// PHP trims *bytes*, so this can produce invalid UTF-8 for inputs that end in
/// a multi-byte character whose continuation byte is `0xA0` (e.g. `€`).
/// Kuukan returns a lossy UTF-8 string instead of panicking.
pub fn utf8_nbsp_trim(string: &str) -> String {
    let bytes = string.as_bytes();
    let mut start = 0;
    let mut end = bytes.len();
    while start < end && (bytes[start] == 0xC2 || bytes[start] == 0xA0) {
        start += 1;
    }
    while end > start && (bytes[end - 1] == 0xC2 || bytes[end - 1] == 0xA0) {
        end -= 1;
    }
    String::from_utf8_lossy(&bytes[start..end]).into_owned()
}

/// Port of `JString::strToCanonical()`.
///
/// `preg_replace("/[^[:alnum:][:space:]\-\/]/u", '', $s)` then replace spaces
/// and slashes with underscores. The `/u` modifier makes PCRE's POSIX classes
/// Unicode aware (`[:alnum:]` keeps `Pokémon`/`日本語`, `[:space:]` keeps
/// NBSP), which `is_alphabetic`/`is_numeric`/`is_whitespace` reproduce.
pub fn str_to_canonical(string: &str) -> String {
    let filtered: String = string
        .chars()
        .filter(|c| {
            c.is_alphabetic() || c.is_numeric() || c.is_whitespace() || *c == '-' || *c == '/'
        })
        .collect();
    filtered.replace([' ', '/'], "_")
}

/// Port of `JString::isStringFloat()`.
pub fn is_string_float(string: &str) -> bool {
    is_numeric_php(string) && string.contains('.')
}

fn php_trim(string: &str) -> String {
    // PHP trim() default charlist: " \t\n\r\0\x0B" (note: no form feed)
    string
        .trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\0' | '\u{0B}'))
        .to_string()
}

/// PHP 8 `is_numeric()` for the decimal/float subset (leading and trailing
/// whitespace allowed since PHP 8.0, no hex/binary/underscores).
fn is_numeric_php(string: &str) -> bool {
    let t = string.trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{0B}' | '\u{0C}'));
    if t.is_empty() {
        return false;
    }
    let bytes = t.as_bytes();
    let mut i = 0;
    if bytes[i] == b'+' || bytes[i] == b'-' {
        i += 1;
        if i == bytes.len() {
            return false;
        }
    }
    let int_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let int_digits = i - int_start;
    let mut frac_digits = 0;
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        let frac_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        frac_digits = i - frac_start;
    }
    if int_digits == 0 && frac_digits == 0 {
        return false;
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let exp_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == exp_start {
            return false;
        }
    }
    i == bytes.len()
}

fn is_php_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0B' | b'\x0C' | b'\r')
}

/// Byte-exact port of PHP's `strip_tags()` (no allowed-tags argument).
///
/// This is `php_strip_tags_ex()` from `ext/standard/string.c` restricted to the
/// `allow == NULL` case, so malformed tags behave exactly like PHP
/// (`"a < b"` is kept, `"a <= b"` strips the rest, `"<script>x</script>"` keeps
/// the text between the tags, HTML comments are dropped, ...).
// The C `switch` cases are kept as nested `if`s on purpose: the port must stay
// line-by-line comparable with `php_strip_tags_ex()`.
#[allow(clippy::collapsible_match)]
pub fn strip_tags(string: &str) -> String {
    let buf = string.as_bytes();
    let end = buf.len();
    let mut out: Vec<u8> = Vec::with_capacity(end);

    // libxml/PHP valid UTF-8 input only loses ASCII bytes, so the output stays
    // valid UTF-8; from_utf8_lossy is only a belt-and-braces fallback.
    let at = |i: usize| -> u8 {
        if i < end {
            buf[i]
        } else {
            0
        }
    };

    let mut p: usize = 0;
    let mut state: u8 = 0;
    let mut depth: i32 = 0;
    let mut in_q: u8 = 0;
    let mut lc: u8 = 0;
    let mut br: i32 = 0;
    let mut is_xml = false;

    while p < end {
        match state {
            0 => {
                let c = buf[p];
                match c {
                    0 => {}
                    b'<' => {
                        if in_q != 0 {
                            // inside quotes: drop
                        } else if is_php_space(at(p + 1)) {
                            out.push(c);
                        } else {
                            lc = b'<';
                            state = 1;
                        }
                    }
                    b'>' => {
                        if depth != 0 {
                            depth -= 1;
                        } else if in_q == 0 {
                            out.push(c);
                        }
                    }
                    _ => out.push(c),
                }
                p += 1;
            }
            1 => {
                let c = buf[p];
                match c {
                    0 => {}
                    b'<' => {
                        if in_q != 0 {
                            // drop
                        } else if is_php_space(at(p + 1)) {
                            // reg_char_1: nothing (no allowed tags)
                        } else {
                            depth += 1;
                        }
                    }
                    b'>' => {
                        if depth != 0 {
                            depth -= 1;
                        } else if in_q == 0 {
                            lc = b'>';
                            if is_xml && p >= 1 && buf[p - 1] == b'-' {
                                // xml comment close; keep state
                            } else {
                                in_q = 0;
                                state = 0;
                                is_xml = false;
                            }
                        }
                    }
                    b'"' | b'\'' => {
                        if p != 0 && (in_q == 0 || c == in_q) {
                            in_q = if in_q != 0 { 0 } else { c };
                        }
                    }
                    b'!' => {
                        if p >= 1 && buf[p - 1] == b'<' {
                            state = 3;
                            lc = c;
                        }
                    }
                    b'?' => {
                        if p >= 1 && buf[p - 1] == b'<' {
                            br = 0;
                            state = 2;
                        }
                    }
                    _ => {}
                }
                p += 1;
            }
            2 => {
                // PHP code: <? ... ?>
                let c = buf[p];
                match c {
                    0 => {}
                    b'(' => {
                        if lc != b'"' && lc != b'\'' {
                            lc = b'(';
                            br += 1;
                        }
                    }
                    b')' => {
                        if lc != b'"' && lc != b'\'' {
                            lc = b')';
                            br -= 1;
                        }
                    }
                    b'>' => {
                        if depth != 0 {
                            depth -= 1;
                        } else if in_q != 0 {
                            // drop
                        } else if br == 0 && p >= 1 && lc != b'"' && buf[p - 1] == b'?' {
                            in_q = 0;
                            state = 0;
                        }
                    }
                    b'"' | b'\'' => {
                        if p >= 1 && buf[p - 1] != b'\\' {
                            if lc == c {
                                lc = 0;
                            } else if lc != b'\\' {
                                lc = c;
                            }
                            if p != 0 && (in_q == 0 || c == in_q) {
                                in_q = if in_q != 0 { 0 } else { c };
                            }
                        }
                    }
                    b'l' | b'L' => {
                        if state == 2
                            && p > 4
                            && (buf[p - 1] == b'm' || buf[p - 1] == b'M')
                            && (buf[p - 2] == b'x' || buf[p - 2] == b'X')
                            && buf[p - 3] == b'?'
                            && buf[p - 4] == b'<'
                        {
                            state = 1;
                            is_xml = true;
                        }
                    }
                    _ => {}
                }
                p += 1;
            }
            3 => {
                // <! ... > (comments, doctype)
                let c = buf[p];
                match c {
                    0 => {}
                    b'>' => {
                        if depth != 0 {
                            depth -= 1;
                        } else if in_q == 0 {
                            in_q = 0;
                            state = 0;
                        }
                    }
                    b'"' | b'\'' => {
                        if p != 0 && buf[p - 1] != b'\\' && (in_q == 0 || c == in_q) {
                            in_q = if in_q != 0 { 0 } else { c };
                        }
                    }
                    b'-' => {
                        if p >= 2 && buf[p - 1] == b'-' && buf[p - 2] == b'!' {
                            state = 4;
                        }
                    }
                    b'E' | b'e' => {
                        // !DOCTYPE exception: switch back to a normal tag
                        if p > 6
                            && (buf[p - 1] == b'p' || buf[p - 1] == b'P')
                            && (buf[p - 2] == b'y' || buf[p - 2] == b'Y')
                            && (buf[p - 3] == b't' || buf[p - 3] == b'T')
                            && (buf[p - 4] == b'c' || buf[p - 4] == b'C')
                            && (buf[p - 5] == b'o' || buf[p - 5] == b'O')
                            && (buf[p - 6] == b'd' || buf[p - 6] == b'D')
                        {
                            state = 1;
                        }
                    }
                    _ => {}
                }
                p += 1;
            }
            4 => {
                // <!-- comment -->: skip until --> or end of string
                while p < end {
                    if buf[p] == b'>'
                        && in_q == 0
                        && p >= 2
                        && buf[p - 1] == b'-'
                        && buf[p - 2] == b'-'
                    {
                        in_q = 0;
                        state = 0;
                        p += 1;
                        break;
                    }
                    p += 1;
                }
            }
            _ => unreachable!(),
        }
    }

    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ported from test/JikanTest/Helper/JStringTest.php
    #[test]
    fn string_float_provider() {
        assert!(is_string_float("3.123"));
        assert!(is_string_float(" 3.123"));
        assert!(!is_string_float(" abc 3.123"));
        assert!(!is_string_float("3..123"));
    }

    #[test]
    fn is_string_float_php8_edge_cases() {
        assert!(is_string_float(".5"));
        assert!(is_string_float("5."));
        assert!(is_string_float("+5.0"));
        assert!(is_string_float("-0."));
        assert!(!is_string_float("+."));
        assert!(!is_string_float("5e3"));
        assert!(!is_string_float("1.2.3"));
        assert!(!is_string_float("0x1A"));
        assert!(is_string_float("\t3.5\n"));
    }

    #[test]
    fn canonical_provider() {
        // Ported verbatim from JStringTest::it_converts_string_to_canonical_format
        let cases = [
            ("Asuka (Monthly)", "Asuka_Monthly"),
            ("5pb.", "5pb"),
            ("3xCube", "3xCube"),
            ("4Kids Entertainment", "4Kids_Entertainment"),
            ("1st PLACE", "1st_PLACE"),
            ("81 Produce", "81_Produce"),
            ("A-1 Pictures", "A-1_Pictures"),
            ("AXsiZ", "AXsiZ"),
            ("F.M.F", "FMF"),
            ("U/M/A/A Inc.", "U_M_A_A_Inc"),
            ("ZIZ Entertainment (ZIZ)", "ZIZ_Entertainment_ZIZ"),
            (".hack//G.U. The World", "hack__GU_The_World"),
            ("C'omma", "Comma"),
        ];
        for (input, expected) in cases {
            assert_eq!(str_to_canonical(input), expected, "input: {input:?}");
        }
    }

    #[test]
    fn canonical_unicode() {
        // /u makes PCRE's POSIX classes Unicode aware
        assert_eq!(str_to_canonical("Pokémon"), "Pokémon");
        assert_eq!(str_to_canonical("日本語"), "日本語");
        assert_eq!(str_to_canonical("Übel"), "Übel");
        assert_eq!(str_to_canonical("a_b"), "ab");
        assert_eq!(str_to_canonical("  "), "__");
    }

    // Expected values captured from PHP 8.5 (`JString::cleanse`).
    #[test]
    fn cleanse_cases() {
        let cases: &[(&str, &str)] = &[
            ("Hello<br>World", "Hello\nWorld"),
            ("Hello<br />World", "Hello\nWorld"),
            ("Hello<br/>World", "Hello\nWorld"),
            ("Hello<br >World", "Hello\nWorld"),
            ("Hello<br >World<br>Again", "Hello\nWorld\nAgain"),
            ("a &amp; b", "a &amp; b"),
            ("a\u{a0}b", "a b"),
            ("  spaced  ", "spaced"),
            ("line1\nline2\ttab", "line1line2tab"),
            ("ctrl\u{1}char", "ctrlchar"),
            ("<b>bold</b> text", "bold text"),
            ("<div>block <span>inline</span></div>", "block inline"),
            ("a < b", "a < b"),
            ("a < b> c", "a < b> c"),
            ("a <1> b", "a  b"),
            ("a <= b", "a"),
            ("a </ b", "a"),
            ("a <!-- comment --> b", "a  b"),
            ("a <script>x</script> b", "a x b"),
            ("a <<b>> c", "a  c"),
            ("a <b c=\"d>e\">f", "a f"),
            ("text\\nliteral", "text\nliteral"),
            ("trail\\n", "trail"),
            ("\\n", ""),
            ("", ""),
            ("\u{7f}del", "del"),
            ("日本<br>語", "日本\n語"),
            ("<br>", ""),
            ("a<br>", "a"),
            ("  <br>  ", ""),
        ];
        for (input, expected) in cases {
            assert_eq!(cleanse(input), *expected, "input: {input:?}");
        }
    }

    // Expected values captured from PHP 8.5 (`strip_tags`).
    #[test]
    fn strip_tags_php_cases() {
        let cases: &[(&str, &str)] = &[
            ("a <b> c", "a  c"),
            ("a <!-- c --> b", "a  b"),
            ("a <script>x</script> b", "a x b"),
            ("a &amp; b", "a &amp; b"),
            ("a <br> b", "a  b"),
            ("<", ""),
            ("a <", "a "),
            ("< ", "< "),
            ("a < b", "a < b"),
            ("<>", ""),
            ("a <> b", "a  b"),
            ("<<b>>", ""),
            ("< b>", "< b>"),
            ("a < b> c", "a < b> c"),
            ("a <1> b", "a  b"),
            ("a <= b", "a "),
            ("a <! b", "a "),
            ("a <!-- b", "a "),
            ("a </ b", "a "),
            ("a <b", "a "),
            ("a <b c", "a "),
            ("x < y < z", "x < y < z"),
            ("a < b > c", "a < b > c"),
            ("< b", "< b"),
            ("a <<b>> c", "a  c"),
            ("a <b c=\"d>e\">f", "a f"),
            ("a <?php echo 1; ?> b", "a  b"),
            ("a <!DOCTYPE html> b", "a  b"),
        ];
        for (input, expected) in cases {
            assert_eq!(strip_tags(input), *expected, "input: {input:?}");
        }
    }

    #[test]
    fn utf8_nbsp_trim_bytes() {
        assert_eq!(utf8_nbsp_trim("\u{a0}x\u{a0}"), "x");
        assert_eq!(utf8_nbsp_trim("x"), "x");
        assert_eq!(utf8_nbsp_trim("  x  "), "  x  ");
        assert_eq!(utf8_nbsp_trim("\u{a0}\u{a0}x\u{a0}\u{a0}"), "x");
    }
}
