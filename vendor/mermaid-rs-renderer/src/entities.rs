//! vendored: Mermaid's `#…;` escapes, decoded.
//!
//! Mermaid lets a label carry a character that would otherwise end it, by
//! writing it as `#` then a name or a number then `;`. Its own documentation
//! gives `#9829;` for a heart and `#infin;` for an infinity sign, and says
//! "numbers given are base 10, so `#` can be encoded as `#35;`. It is also
//! supported to use HTML character names."
//!
//! Upstream read none of them. The only entity handling in this renderer was
//! `escape_xml` on the way out, so a label that carried an escape drew the
//! escape: a heading with a quotation mark in it came out as
//! `He said #quot;no#quot;`. It matters most to a caller whose node text is
//! taken from a document, because a `"` in that text has to be written
//! `#quot;`: a bare one would close the bracketed label it is being put
//! inside.
//!
//! # Where this runs
//!
//! At measurement, on the way into a [`TextBlock`](crate::layout::TextBlock) --
//! which is what the renderer draws from, and what every diagram kind's text
//! passes through. Not in the parser: the escape exists so that a `"` or a `]`
//! cannot terminate a label, so it has to survive until the brackets around it
//! have been consumed. Decoding earlier would reintroduce the problem the
//! escape is there to prevent.
//!
//! # What is and is not an escape
//!
//! Only `#`, then a run of letters and digits, then `;`. A `#` on its own is a
//! `#`, `#foo` with no semicolon is `#foo`, and a name this table does not
//! know is left exactly as written rather than dropped -- the safe failure for
//! text that was never an escape to begin with.

use std::borrow::Cow;

/// HTML character names, as Mermaid's documentation calls them.
///
/// Mermaid hands the name to the browser, so upstream's set is HTML's whole
/// named-character-reference table, some two thousand entries. This is the part
/// of it a document heading reaches for: the characters that cannot be written
/// literally inside a label, the punctuation a writer actually types, and the
/// symbols Mermaid's own examples use. A name outside it is left as written, so
/// the cost of the difference is visible rather than silent.
const NAMED: &[(&str, &str)] = &[
    // The ones that have to be escaped to appear at all.
    ("quot", "\""),
    ("apos", "'"),
    ("amp", "&"),
    ("lt", "<"),
    ("gt", ">"),
    ("hash", "#"),
    ("semi", ";"),
    ("colon", ":"),
    ("equals", "="),
    ("lbrace", "{"),
    ("rbrace", "}"),
    ("lbrack", "["),
    ("rbrack", "]"),
    ("lpar", "("),
    ("rpar", ")"),
    ("verbar", "|"),
    ("sol", "/"),
    ("bsol", "\\"),
    ("grave", "`"),
    ("ast", "*"),
    ("plus", "+"),
    ("dash", "-"),
    ("num", "#"),
    ("percnt", "%"),
    ("dollar", "$"),
    ("commat", "@"),
    ("excl", "!"),
    ("quest", "?"),
    ("period", "."),
    ("comma", ","),
    ("lowbar", "_"),
    ("tilde", "~"),
    ("Hat", "^"),
    // Spaces.
    ("nbsp", "\u{00A0}"),
    ("ensp", "\u{2002}"),
    ("emsp", "\u{2003}"),
    ("thinsp", "\u{2009}"),
    // The punctuation a writer types, and a word processor substitutes.
    ("ndash", "\u{2013}"),
    ("mdash", "\u{2014}"),
    ("lsquo", "\u{2018}"),
    ("rsquo", "\u{2019}"),
    ("ldquo", "\u{201C}"),
    ("rdquo", "\u{201D}"),
    ("sbquo", "\u{201A}"),
    ("bdquo", "\u{201E}"),
    ("hellip", "\u{2026}"),
    ("prime", "\u{2032}"),
    ("Prime", "\u{2033}"),
    ("laquo", "\u{00AB}"),
    ("raquo", "\u{00BB}"),
    ("lsaquo", "\u{2039}"),
    ("rsaquo", "\u{203A}"),
    ("bull", "\u{2022}"),
    ("middot", "\u{00B7}"),
    ("dagger", "\u{2020}"),
    ("Dagger", "\u{2021}"),
    ("sect", "\u{00A7}"),
    ("para", "\u{00B6}"),
    // Marks and signs.
    ("copy", "\u{00A9}"),
    ("reg", "\u{00AE}"),
    ("trade", "\u{2122}"),
    ("deg", "\u{00B0}"),
    ("permil", "\u{2030}"),
    ("micro", "\u{00B5}"),
    ("pound", "\u{00A3}"),
    ("euro", "\u{20AC}"),
    ("yen", "\u{00A5}"),
    ("cent", "\u{00A2}"),
    ("curren", "\u{00A4}"),
    // Arithmetic and comparison.
    ("plusmn", "\u{00B1}"),
    ("times", "\u{00D7}"),
    ("divide", "\u{00F7}"),
    ("minus", "\u{2212}"),
    ("ne", "\u{2260}"),
    ("le", "\u{2264}"),
    ("ge", "\u{2265}"),
    ("asymp", "\u{2248}"),
    ("equiv", "\u{2261}"),
    ("infin", "\u{221E}"),
    ("radic", "\u{221A}"),
    ("sum", "\u{2211}"),
    ("prod", "\u{220F}"),
    ("part", "\u{2202}"),
    ("frac12", "\u{00BD}"),
    ("frac14", "\u{00BC}"),
    ("frac34", "\u{00BE}"),
    ("sup2", "\u{00B2}"),
    ("sup3", "\u{00B3}"),
    // Arrows, which a diagram's prose reaches for often.
    ("larr", "\u{2190}"),
    ("rarr", "\u{2192}"),
    ("uarr", "\u{2191}"),
    ("darr", "\u{2193}"),
    ("harr", "\u{2194}"),
    ("lArr", "\u{21D0}"),
    ("rArr", "\u{21D2}"),
    ("hArr", "\u{21D4}"),
    // Card suits and marks: Mermaid's own example is `#9829;`, the heart.
    ("hearts", "\u{2665}"),
    ("diams", "\u{2666}"),
    ("clubs", "\u{2663}"),
    ("spades", "\u{2660}"),
    ("check", "\u{2713}"),
    ("cross", "\u{2717}"),
    ("star", "\u{2606}"),
    ("starf", "\u{2605}"),
    // Greek, for the documents that need it.
    ("alpha", "\u{03B1}"),
    ("beta", "\u{03B2}"),
    ("gamma", "\u{03B3}"),
    ("delta", "\u{03B4}"),
    ("epsilon", "\u{03B5}"),
    ("theta", "\u{03B8}"),
    ("lambda", "\u{03BB}"),
    ("mu", "\u{03BC}"),
    ("pi", "\u{03C0}"),
    ("sigma", "\u{03C3}"),
    ("tau", "\u{03C4}"),
    ("phi", "\u{03C6}"),
    ("omega", "\u{03C9}"),
    ("Delta", "\u{0394}"),
    ("Sigma", "\u{03A3}"),
    ("Omega", "\u{03A9}"),
];

/// Resolve one escape's body -- what stood between the `#` and the `;`.
///
/// `None` means it is not an escape this knows, and the caller puts the text
/// back as it found it.
fn resolve(body: &str) -> Option<&'static str> {
    if body.is_empty() {
        return None;
    }
    if let Some(name) = NAMED
        .iter()
        .find_map(|(name, value)| (*name == body).then_some(*value))
    {
        return Some(name);
    }
    None
}

/// Whether XML can carry this character at all.
///
/// The output is an SVG document, so a decoded control character would not be
/// badly drawn, it would make the file unparseable. `#1;` is a number this can
/// read and must still refuse, and refusing means leaving it as written.
fn xml_allows(c: char) -> bool {
    matches!(c,
        '\t' | '\n' | '\r'
        | '\u{20}'..='\u{D7FF}'
        | '\u{E000}'..='\u{FFFD}'
        | '\u{10000}'..='\u{10FFFF}')
}

/// Mermaid's escapes in `text`, replaced by the characters they stand for.
///
/// Borrows when there is nothing to do, which is almost every label.
pub fn decode(text: &str) -> Cow<'_, str> {
    if !text.contains('#') {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut decoded_any = false;

    while let Some(hash) = rest.find('#') {
        out.push_str(&rest[..hash]);
        let after = &rest[hash + 1..];
        // The body runs to the first `;`, and is letters and digits only --
        // anything else means this `#` was just a `#`.
        let end = after
            .find(';')
            .filter(|end| after[..*end].chars().all(|c| c.is_ascii_alphanumeric()));
        let Some(end) = end else {
            out.push('#');
            rest = after;
            continue;
        };
        let body = &after[..end];
        // A number is base 10, as Mermaid's documentation says.
        let decoded = if body.chars().all(|c| c.is_ascii_digit()) {
            body.parse::<u32>()
                .ok()
                .and_then(char::from_u32)
                .filter(|c| xml_allows(*c))
                .map(|c| c.to_string())
        } else {
            resolve(body).map(str::to_string)
        };
        match decoded {
            Some(value) => {
                out.push_str(&value);
                decoded_any = true;
            }
            // Not one this knows: leave it exactly as written.
            None => {
                out.push('#');
                out.push_str(body);
                out.push(';');
            }
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);

    if decoded_any {
        Cow::Owned(out)
    } else {
        Cow::Borrowed(text)
    }
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn a_named_escape_becomes_its_character() {
        assert_eq!(decode("He said #quot;no#quot;"), "He said \"no\"");
        assert_eq!(decode("I #hearts; you"), "I \u{2665} you");
        assert_eq!(decode("#infin; times more"), "\u{221E} times more");
        assert_eq!(decode("a #amp; b"), "a & b");
    }

    #[test]
    fn a_number_is_base_ten() {
        // Mermaid's own two examples.
        assert_eq!(decode("I #9829; you!"), "I \u{2665} you!");
        assert_eq!(decode("#35;"), "#");
        // A semicolon, which ends a line in a sequence diagram otherwise.
        assert_eq!(decode("Value#59; Key"), "Value; Key");
        // Not hexadecimal: the documentation says base 10, so `#x41;` is a
        // name this does not know, and stays as written.
        assert_eq!(decode("#x41;"), "#x41;");
    }

    #[test]
    fn a_literal_hash_survives() {
        assert_eq!(decode("#"), "#");
        assert_eq!(decode("C# and F#"), "C# and F#");
        assert_eq!(decode("issue #42 is open"), "issue #42 is open");
        assert_eq!(decode("#foo"), "#foo");
        assert_eq!(decode("#;"), "#;");
        assert_eq!(decode("##"), "##");
        // A body with punctuation in it was never an escape.
        assert_eq!(decode("#not-a-name;"), "#not-a-name;");
        // A number XML cannot carry is refused and left as written, or the
        // SVG would not parse.
        assert_eq!(decode("see #1; and #2;"), "see #1; and #2;");
        assert_eq!(decode("#0;"), "#0;");
        assert_eq!(decode("#65533;"), "\u{FFFD}");
        assert_eq!(decode("#1114112;"), "#1114112;");
    }

    #[test]
    fn an_unknown_name_is_left_as_written() {
        // HTML has some two thousand names and this table has a hundred; the
        // ones it misses have to come through unharmed rather than vanish.
        assert_eq!(decode("#thereisnosuchentity;"), "#thereisnosuchentity;");
    }

    #[test]
    fn text_without_an_escape_is_borrowed() {
        assert!(matches!(
            decode("nothing to do here"),
            std::borrow::Cow::Borrowed(_)
        ));
        assert!(matches!(decode("C# alone"), std::borrow::Cow::Borrowed(_)));
    }

    #[test]
    fn several_escapes_in_one_label() {
        assert_eq!(decode("#quot;a#quot; #amp; #quot;b#quot;"), "\"a\" & \"b\"");
    }
}
