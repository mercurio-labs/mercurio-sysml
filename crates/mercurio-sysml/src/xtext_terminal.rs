//! Definition-driven recognition of all pinned Xtext terminal bodies.
//!
//! This recognizes a selected rule's prefix. ANTLR DFA prediction among rules,
//! hidden-token policy, parser-rule number grouping, and value conversion are
//! separate contracts. FIRST/FOLLOW checks qualify committed body decisions;
//! lexer arbitration still uses its independently exported ANTLR tables.

#[derive(Clone, Copy)]
enum Cardinality {
    One,
    Optional,
    ZeroOrMore,
    OneOrMore,
}
#[derive(Clone, Copy)]
enum Operation {
    Literal(&'static [u16]),
    Range(u16, u16),
    Sequence(&'static [usize]),
    Choice(&'static [usize]),
    NotCharacters(&'static [u16]),
    Until(&'static [u16]),
    Call(usize),
}
#[derive(Clone, Copy)]
struct Node {
    op: Operation,
    cardinality: Cardinality,
    first: &'static [(u16, u16)],
}
struct Terminal {
    name: &'static str,
    id: &'static str,
    program: usize,
}
#[path = "xtext_terminal_generated.rs"]
mod generated;

struct Matcher<'a> {
    remaining: std::str::EncodeUtf16<'a>,
    units: Vec<u16>,
}

impl<'a> Matcher<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            remaining: input.encode_utf16(),
            units: Vec::new(),
        }
    }

    // Read only the prefix that this rule inspects. Converting the entire
    // remaining source at each quoted token would create quadratic work.
    fn unit(&mut self, offset: usize) -> Option<u16> {
        while self.units.len() <= offset {
            self.units.push(self.remaining.next()?);
        }
        self.units.get(offset).copied()
    }

    fn literal(&mut self, offset: usize, text: &[u16]) -> bool {
        text.iter()
            .enumerate()
            .all(|(i, value)| self.unit(offset + i) == Some(*value))
    }

    fn starts(&mut self, index: usize, offset: usize) -> bool {
        self.unit(offset).is_some_and(|unit| {
            generated::PROGRAM[index]
                .first
                .iter()
                .any(|(low, high)| (*low..=*high).contains(&unit))
        })
    }

    // Generation proves disjoint FIRST sets and greedy exit decisions. Once
    // entry is selected, failure is committed; optionality cannot hide it.
    fn matched(&mut self, index: usize, offset: usize) -> Option<usize> {
        let node = generated::PROGRAM[index];
        match node.cardinality {
            Cardinality::One => self.body(node.op, offset),
            Cardinality::Optional => {
                if self.starts(index, offset) {
                    self.body(node.op, offset)
                } else {
                    Some(offset)
                }
            }
            Cardinality::ZeroOrMore | Cardinality::OneOrMore => {
                let mut cursor = offset;
                while self.starts(index, cursor) {
                    let end = self.body(node.op, cursor)?;
                    if end <= cursor {
                        return None;
                    }
                    cursor = end;
                }
                (cursor > offset || matches!(node.cardinality, Cardinality::ZeroOrMore))
                    .then_some(cursor)
            }
        }
    }

    fn body(&mut self, operation: Operation, offset: usize) -> Option<usize> {
        match operation {
            Operation::Literal(text) => self.literal(offset, text).then_some(offset + text.len()),
            Operation::Range(left, right) => self
                .unit(offset)
                .filter(|value| (left..=right).contains(value))
                .map(|_| offset + 1),
            Operation::NotCharacters(excluded) => self
                .unit(offset)
                .filter(|value| !excluded.contains(value))
                .map(|_| offset + 1),
            Operation::Until(delimiter) => {
                let mut current = offset;
                while self.unit(current).is_some() {
                    if self.literal(current, delimiter) {
                        return Some(current + delimiter.len());
                    }
                    current += 1;
                }
                None
            }
            Operation::Call(target) => self.matched(target, offset),
            Operation::Choice(children) => {
                let child = children.iter().find(|child| self.starts(**child, offset))?;
                self.matched(*child, offset)
            }
            Operation::Sequence(children) => {
                let mut cursor = offset;
                for child in children {
                    cursor = self.matched(*child, cursor)?;
                }
                Some(cursor)
            }
        }
    }
}

/// Return the UTF-8 byte length consumed by a committed terminal program.
/// The program executes UTF-16 code units, matching the upstream terminal domain.
/// Unknown rule names and rejected prefixes return None; no fallback is selected.
pub(crate) fn terminal_prefix(rule: &str, input: &str) -> Option<usize> {
    let terminal = generated::TERMINALS
        .iter()
        .find(|row| row.name == rule || row.id == rule)?;
    let end = Matcher::new(input).matched(terminal.program, 0)?;
    let mut utf16 = 0;
    if end == 0 {
        return Some(0);
    }
    for (byte, character) in input.char_indices() {
        utf16 += character.len_utf16();
        if utf16 == end {
            return Some(byte + character.len_utf8());
        }
        if utf16 > end {
            return None;
        }
    }
    None
}

/// Exact parser-keyword membership after inherited rules and overrides resolve.
/// This does not choose a token over ID or execute a parser rule.
pub(crate) fn is_keyword(kerml: bool, spelling: &str) -> bool {
    let keywords = if kerml {
        generated::KERML_KEYWORDS
    } else {
        generated::SYSML_KEYWORDS
    };
    keywords.binary_search(&spelling).is_ok()
}

#[path = "xtext_lexer_decision.rs"]
mod lexer_decision;

pub(crate) fn keyword_limit(kerml: bool) -> Result<usize, &'static str> {
    lexer_decision::keyword_limit(kerml)
}

/// Use the complete language lexer decision, independent of parser context.
pub(crate) fn keyword_prefix(
    kerml: bool,
    input: &str,
) -> Result<Option<&'static str>, &'static str> {
    lexer_decision::keyword_prefix(kerml, input)
}

/// Split the legacy Number carrier using upstream token prediction, not a
/// longest-match competition between independently recognized terminal bodies.
/// The carrier boundary is handwritten; token identity and body are imported.
pub(crate) fn numeric_component(input: &str) -> Result<(String, usize), &'static str> {
    let label = lexer_decision::predict(input)?;
    if let Some(rule) = label.strip_prefix("RULE_") {
        if !matches!(rule, "DECIMAL_VALUE" | "EXP_VALUE") {
            return Err("non-numeric terminal in Number carrier");
        }
        let length = terminal_prefix(rule, input)
            .filter(|length| *length > 0)
            .ok_or("predicted numeric terminal has no recognized body")?;
        return Ok((format!("terminal:{rule}"), length));
    }
    let keyword = lexer_decision::keyword(label)?.ok_or("unknown numeric keyword identity")?;
    if keyword != "." || !input.starts_with(keyword) {
        return Err("non-numeric keyword in Number carrier");
    }
    Ok((format!("keyword:{keyword}"), keyword.len()))
}

/// The pin shares one inherited hidden-token policy across all three grammars.
/// Recognition comes from generated terminal bodies, selection from exported
/// ANTLR tables. Comment value adaptation remains explicit handwritten behavior.
fn hidden_prefix(
    input: &str,
) -> Result<Option<mercurio_foundation::language_contracts::lexer::HiddenTokenMatch>, &'static str>
{
    use mercurio_foundation::language_contracts::ast::CommentKind;
    use mercurio_foundation::language_contracts::lexer::HiddenTokenMatch;
    if input.is_empty() {
        return Ok(None);
    }
    // Imported FIRST sets identify possible hidden starts even when a selected
    // body later fails. Testing complete recognition here could hide a committed
    // error by falling back to visible punctuation.
    if !generated::HIDDEN_TOKENS.iter().any(|id| {
        generated::TERMINALS
            .iter()
            .find(|row| row.id == *id)
            .is_some_and(|row| Matcher::new(input).starts(row.program, 0))
    }) {
        return Ok(None);
    }
    let selected = lexer_decision::predict(input)?;
    let selected = selected.strip_prefix("RULE_").unwrap_or(selected);
    for identity in generated::HIDDEN_TOKENS {
        let name = identity
            .rsplit("::")
            .next()
            .ok_or("invalid hidden terminal identity")?;
        if name != selected {
            continue;
        }
        let length = terminal_prefix(identity, input)
            .ok_or("predicted hidden token has no recognized body")?;
        let comment = match name {
            "WS" => None,
            "ML_NOTE" => Some((CommentKind::Block, 3..length - 2)),
            "SL_NOTE" => {
                let end = input[..length].trim_end_matches(['\r', '\n']).len();
                Some((CommentKind::Line, 2..end))
            }
            _ => return Err("unsupported hidden terminal adapter"),
        };
        return Ok(Some(HiddenTokenMatch { length, comment }));
    }
    Ok(None)
}

/// Preserve the shared token adapter while deriving delimited terminal bounds.
pub(crate) fn lex(
    input: &str,
) -> Result<
    Vec<mercurio_foundation::language_contracts::lexer::Token>,
    mercurio_foundation::language_contracts::Diagnostic,
> {
    mercurio_foundation::language_contracts::lexer::lex_with_numeric_matcher(
        input,
        terminal_prefix,
        hidden_prefix,
        crate::xtext_fragment::numeric_prefix,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_selection_uses_language_and_full_lexer_context() {
        assert_eq!(keyword_prefix(false, "part"), Ok(Some("part")));
        assert_eq!(keyword_prefix(true, "part"), Ok(None));
        for kerml in [false, true] {
            assert_eq!(keyword_prefix(kerml, "package_name"), Ok(None));
            assert_eq!(keyword_prefix(kerml, "package x"), Ok(Some("package")));
            assert_eq!(keyword_prefix(kerml, "===x"), Ok(Some("===")));
            assert_eq!(keyword_prefix(kerml, "::>x"), Ok(Some("::>")));
            assert_eq!(keyword_prefix(kerml, "..2"), Ok(Some("..")));
        }
    }

    #[test]
    fn generated_identifiers_and_semantic_comments_preserve_source_and_trivia() {
        use mercurio_foundation::language_contracts::lexer::TokenKind;
        let source = "// note\nalpha_9 /*a\n😀*/ beta";
        let tokens = lex(source).unwrap();
        assert_eq!(tokens.len(), 4);
        assert!(matches!(&tokens[0].kind, TokenKind::Identifier(name) if name == "alpha_9"));
        assert_eq!(tokens[0].leading_trivia.len(), 1);
        assert_eq!(tokens[0].span.start_line, 2);
        assert!(matches!(&tokens[1].kind, TokenKind::BlockDoc(text) if text == "a\n😀"));
        assert_eq!(tokens[1].span.start_line, 2);
        assert_eq!(tokens[1].span.end_line, 3);
        assert!(matches!(&tokens[2].kind, TokenKind::Identifier(name) if name == "beta"));
        assert!(lex("/*unterminated").is_err());
        assert!(lex("// note\rrest").is_err());
        assert!(lex("alphaé").is_err());
        let adjacent = lex("/**//**/word").unwrap();
        assert!(matches!(&adjacent[0].kind, TokenKind::BlockDoc(text) if text.is_empty()));
        assert!(matches!(&adjacent[1].kind, TokenKind::BlockDoc(text) if text.is_empty()));
        assert!(matches!(&adjacent[2].kind, TokenKind::Identifier(name) if name == "word"));
    }

    #[test]
    fn committed_terminal_programs_match_independent_pilot_bodies() {
        let doc: serde_json::Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/lexer-decision.extract.json"
        ))
        .unwrap();
        let controls = doc["terminal_body_controls"].as_array().unwrap();
        assert!(controls.len() > 1200);
        let mut rules = std::collections::BTreeSet::new();
        for case in controls {
            let rule = case["rule"].as_str().unwrap();
            let source = case["source"].as_str().unwrap();
            rules.insert(rule);
            let actual = terminal_prefix(rule, source)
                .map(|end| source[..end].encode_utf16().count() as u64);
            assert_eq!(actual, case["length_utf16"].as_u64(), "{rule} {source:?}");
        }
        assert_eq!(rules.len(), generated::TERMINALS.len());
    }

    #[test]
    fn imported_numeric_syntax_drives_source_carriers_and_range_boundaries() {
        use mercurio_foundation::language_contracts::lexer::TokenKind;
        for source in ["0", "1.25", ".5", "12e-3", "12.5E+3", "2147483648.5"] {
            let tokens = lex(source).unwrap();
            assert_eq!(tokens.len(), 2, "{source}");
            assert!(matches!(&tokens[0].kind, TokenKind::Number(value) if value == source));
            assert_eq!(tokens[0].span.end_col, source.len());
        }
        for source in ["1e", "1e+", "1.2e-", ".5e+"] {
            assert!(lex(source).is_err(), "{source}");
        }
        let tokens = lex("1..2").unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Number(value) if value == "1"));
        assert!(matches!(tokens[1].kind, TokenKind::Dot));
        assert!(matches!(tokens[2].kind, TokenKind::Dot));
        assert!(matches!(&tokens[3].kind, TokenKind::Number(value) if value == "2"));
    }

    #[test]
    fn numeric_components_match_independent_pilot_tokens_and_committed_errors() {
        let doc: serde_json::Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/lexer-decision.extract.json"
        ))
        .unwrap();
        for case in doc["numeric_token_controls"].as_array().unwrap() {
            let source = case["source"].as_str().unwrap();
            assert_eq!(
                lex(source).is_err(),
                case["errors"].as_u64().unwrap() > 0,
                "source acceptance: {source:?}"
            );
            // Range punctuation lies outside a Number carrier; its separate
            // public-token representation remains a handwritten adapter.
            if source.contains("..") {
                continue;
            }
            let mut cursor = 0;
            let mut failed = false;
            let mut actual = Vec::new();
            while cursor < source.len() {
                let tail = &source[cursor..];
                // The numeric carrier ends before identifiers following a number.
                if !tail.starts_with('.') && !tail.as_bytes()[0].is_ascii_digit() {
                    break;
                }
                match numeric_component(tail) {
                    Ok((symbol, length)) => {
                        let name = symbol
                            .strip_prefix("terminal:")
                            .map(|name| format!("RULE_{name}"))
                            .unwrap_or_else(|| format!("'{}'", &symbol[8..]));
                        actual.push(serde_json::json!({"name":name,"text":&tail[..length]}));
                        cursor += length;
                    }
                    Err(_) => {
                        failed = true;
                        break;
                    }
                }
            }
            assert_eq!(failed, case["errors"].as_u64().unwrap() > 0, "{source:?}");
            if !failed {
                let expected = case["tokens"].as_array().unwrap();
                assert_eq!(actual.as_slice(), &expected[..actual.len()], "{source:?}");
                if cursor == source.len() {
                    assert_eq!(actual.len(), expected.len(), "{source:?}");
                }
            }
        }
        // Unlike trying DECIMAL_VALUE first, the imported decision commits to
        // EXP_VALUE and cannot silently accept the leading integer.
        assert_eq!(terminal_prefix("DECIMAL_VALUE", "1e+"), Some(1));
        assert!(numeric_component("1e+").is_err());
        assert!(numeric_component("..").is_err());
    }

    #[test]
    fn inherited_hidden_policy_matches_pilot_prediction_acceptance_and_spans() {
        let evidence: serde_json::Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-prediction-nfa.experimental.json"
        ))
        .unwrap();
        let cases = evidence["hidden_token_controls"].as_array().unwrap();
        assert_eq!(cases.len(), 16);
        let mut matches = 0;
        for case in cases {
            let source = case["source"].as_str().unwrap();
            let context = case["context"].as_str().unwrap();
            let accepted = crate::xtext_fragment::experimental_model(
                source,
                &format!("{context}::Package"),
                context.ends_with(".KerML"),
            )
            .is_ok();
            assert_eq!(accepted, case["accepted"].as_bool().unwrap(), "{source:?}");
            matches += 1;
            if !accepted {
                continue;
            }
            for leaf in case["hidden"].as_array().unwrap() {
                let offset = leaf["offset"].as_u64().unwrap() as usize;
                // These controls use BMP/ASCII text, so upstream UTF-16 offsets
                // equal byte offsets on every accepted source.
                let matched = hidden_prefix(&source[offset..]).unwrap().unwrap();
                assert_eq!(
                    matched.length,
                    leaf["length"].as_u64().unwrap() as usize,
                    "{source:?} {leaf}"
                );
                assert_eq!(
                    &source[offset..offset + matched.length],
                    leaf["text"].as_str().unwrap()
                );
            }
        }
        assert_eq!(matches, 16);
        for whitespace in ["\u{00a0}", "\u{000b}", "\u{000c}"] {
            assert!(lex(&format!("package{whitespace}P {{}}")).is_err());
        }
        assert!(matches!(
            lex("/*semantic*/").unwrap()[0].kind,
            mercurio_foundation::language_contracts::lexer::TokenKind::BlockDoc(_)
        ));
    }

    #[test]
    fn every_pinned_terminal_has_positive_and_negative_recognition_controls() {
        let controls = [
            ("DECIMAL_VALUE", "0123x", Some(4), "x", None),
            ("EXP_VALUE", "12e-34x", Some(6), "12e-", None),
            ("ID", "a_9!", Some(3), "é", None),
            (
                "UNRESTRICTED_NAME",
                "'a\\'b'tail",
                Some(6),
                "'bad\\x'",
                None,
            ),
            (
                "STRING_VALUE",
                "\"a\\\"b\"tail",
                Some(6),
                "\"bad\\u1234\"",
                None,
            ),
            ("REGULAR_COMMENT", "/*a*/tail/*b*/", Some(5), "/*open", None),
            ("ML_NOTE", "//*a*/tail", Some(6), "//*open", None),
            ("SL_NOTE", "//x\r\ntail", Some(5), "/x", None),
            ("WS", " \t\r\nx", Some(4), "\u{a0}", None),
        ];
        assert_eq!(generated::TERMINALS.len(), controls.len());
        for (name, accepted, expected, rejected, absent) in controls {
            assert_eq!(
                terminal_prefix(name, accepted),
                expected,
                "{name}: {accepted:?}"
            );
            assert_eq!(
                terminal_prefix(name, rejected),
                absent,
                "{name}: {rejected:?}"
            );
            let qualified = format!("org.omg.kerml.expressions.xtext.KerMLExpressions::{name}");
            assert_eq!(terminal_prefix(&qualified, accepted), expected);
        }
        assert_eq!(terminal_prefix("unknown", "a"), None);
    }

    #[test]
    fn shared_terminal_operations_preserve_boundaries_eof_and_utf16() {
        for rule in ["UNRESTRICTED_NAME", "STRING_VALUE"] {
            let delimiter = if rule == "UNRESTRICTED_NAME" {
                '\''
            } else {
                '"'
            };
            for escape in ['b', 't', 'n', 'f', 'r', '"', '\'', '\\'] {
                let text = format!("{delimiter}\\{escape}{delimiter}");
                assert_eq!(terminal_prefix(rule, &text), Some(text.len()));
            }
            for body in ["", "😀", "α\nβ", "line\r\nnext"] {
                let text = format!("{delimiter}{body}{delimiter}");
                assert_eq!(terminal_prefix(rule, &text), Some(text.len()));
            }
            assert_eq!(terminal_prefix(rule, &delimiter.to_string()), None);
        }
        for input in ["😀", "9a", "é"] {
            assert_eq!(terminal_prefix("ID", input), None);
        }
        for input in ["\u{b}", "\u{c}", "\u{a0}"] {
            assert_eq!(terminal_prefix("WS", input), None);
        }
        assert_eq!(terminal_prefix("SL_NOTE", "//"), Some(2));
        assert_eq!(terminal_prefix("SL_NOTE", "//x\rrest"), None);
        assert_eq!(
            terminal_prefix("REGULAR_COMMENT", "/* outer /* inner */ tail */"),
            Some(20)
        );
        assert_eq!(terminal_prefix("DECIMAL_VALUE", ".5"), None);
        assert_eq!(terminal_prefix("DECIMAL_VALUE", "1.5"), Some(1));
        assert_eq!(terminal_prefix("EXP_VALUE", "1E+2"), Some(4));
        assert_eq!(terminal_prefix("EXP_VALUE", "1e"), None);
    }

    #[test]
    fn inherited_keyword_inventory_is_exact_and_language_specific() {
        for (kerml, keywords) in [
            (true, generated::KERML_KEYWORDS),
            (false, generated::SYSML_KEYWORDS),
        ] {
            for keyword in keywords {
                assert!(is_keyword(kerml, keyword));
            }
            assert!(!is_keyword(kerml, "keywordThatDoesNotExist"));
            assert!(!is_keyword(kerml, "packageX"));
            for keyword in generated::KERMLEXPRESSIONS_KEYWORDS {
                assert!(is_keyword(kerml, keyword));
            }
        }
        assert!(is_keyword(false, "part"));
        assert!(!is_keyword(true, "part"));
    }

    #[test]
    fn delimited_adapter_preserves_shared_tokens_spans_and_source_spelling() {
        for source in [
            "package P { attribute 'name with spaces' = \"raw\\ntext\"; }",
            "// leading\npackage P { attribute '😀' = \"a\nβ\"; }",
            "/* regular */ package P { attribute x = \"a\\\"b\"; }",
            "package P { attribute '' = \"\"; }",
        ] {
            assert_eq!(
                lex(source).unwrap(),
                mercurio_foundation::language_contracts::lexer::lex(source).unwrap()
            );
        }
        let tokens = lex("'a\\'b'").unwrap();
        assert_eq!(
            tokens[0].kind,
            mercurio_foundation::language_contracts::lexer::TokenKind::Identifier("a\\'b".into())
        );
        assert!(lex("\"bad\\x\"").is_err());
        assert!(lex("'unterminated").is_err());
    }

    #[test]
    fn quoted_recognition_reads_only_the_selected_prefix() {
        let mut input = String::from("\"short\"");
        input.push_str(&"x".repeat(100_000));
        let terminal = generated::TERMINALS
            .iter()
            .find(|r| r.name == "STRING_VALUE")
            .unwrap();
        let mut matcher = Matcher::new(&input);
        assert_eq!(matcher.matched(terminal.program, 0), Some(7));
        assert!(
            matcher.units.len() <= 8,
            "recognizer read unrelated source suffix"
        );
    }
}
