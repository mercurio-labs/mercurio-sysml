//! Native ANTLR token prediction over build-time exported upstream tables.
//! Used for hidden-token and numeric-component selection; carrier construction
//! remains a separate adapter. No Java, file access, or Java interpretation.
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::LazyLock;

#[derive(Deserialize)]
struct Decision {
    #[serde(default)]
    kerml: Option<Box<Decision>>,
    eot: Vec<i32>,
    eof: Vec<i32>,
    min: Vec<u16>,
    max: Vec<u16>,
    accept: Vec<i32>,
    special: Vec<i32>,
    transition: Vec<Vec<i32>>,
    special_ranges: BTreeMap<String, Vec<[i32; 3]>>,
    labels: Vec<String>,
    keywords: BTreeMap<String, String>,
}
static DECISION: LazyLock<Result<Decision, serde_json::Error>> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../resources/metamodels/sysml-2.0-pilot-2026-08/lexer-decision.extract.json"
    ))
});

fn decision(kerml: bool) -> Result<&'static Decision, &'static str> {
    let d = DECISION
        .as_ref()
        .map_err(|_| "invalid imported lexer decision")?;
    if kerml {
        d.kerml.as_deref().ok_or("missing KerML lexer decision")
    } else {
        Ok(d)
    }
}

pub(super) fn keyword(label: &str) -> Result<Option<&'static str>, &'static str> {
    Ok(decision(false)?.keywords.get(label).map(String::as_str))
}

pub(super) fn keyword_limit(kerml: bool) -> Result<usize, &'static str> {
    decision(kerml)?
        .keywords
        .values()
        .map(String::len)
        .max()
        .ok_or("empty keyword inventory")
}

pub(super) fn keyword_prefix(
    kerml: bool,
    input: &str,
) -> Result<Option<&'static str>, &'static str> {
    let label = predict_in_language(kerml, input)?;
    Ok(decision(kerml)?.keywords.get(label).map(String::as_str))
}

pub(super) fn predict(input: &str) -> Result<&'static str, &'static str> {
    predict_in_language(false, input)
}

fn predict_in_language(kerml: bool, input: &str) -> Result<&'static str, &'static str> {
    let d = decision(kerml)?;
    let mut units = input.encode_utf16().peekable();
    let mut state = 0usize;
    for _ in 0..1_000_000 {
        let special = *d.special.get(state).ok_or("invalid lexer state")?;
        if special >= 0 {
            let ranges = d
                .special_ranges
                .get(&special.to_string())
                .ok_or("missing special lexer transition")?;
            let unit = units.peek().map_or(-1, |u| i32::from(*u));
            let target = ranges
                .iter()
                .find(|r| r[0] <= unit && unit <= r[1])
                .ok_or("incomplete special lexer domain")?[2];
            state = usize::try_from(target).map_err(|_| "no viable lexer alternative")?;
            units.next();
            continue;
        }
        let accept = *d.accept.get(state).ok_or("invalid lexer acceptance")?;
        if accept > 0 {
            return d
                .labels
                .get((accept - 1) as usize)
                .map(String::as_str)
                .ok_or("unknown lexer alternative");
        }
        // ANTLR casts LA(1) to char before ordinary table lookup; EOF is 0xffff.
        let unit = units.peek().copied().unwrap_or(u16::MAX);
        let min = *d.min.get(state).ok_or("missing lexer lower bound")?;
        let max = *d.max.get(state).ok_or("missing lexer upper bound")?;
        let eot = *d.eot.get(state).ok_or("missing lexer EOT")?;
        if min <= unit && unit <= max {
            let target = *d
                .transition
                .get(state)
                .and_then(|row| row.get(usize::from(unit - min)))
                .ok_or("missing lexer transition")?;
            if target >= 0 {
                state = target as usize;
                units.next();
                continue;
            }
            if eot >= 0 {
                state = eot as usize;
                units.next();
                continue;
            }
            return Err("no viable lexer alternative");
        }
        if eot >= 0 {
            state = eot as usize;
            units.next();
            continue;
        }
        let eof = *d.eof.get(state).ok_or("missing lexer EOF")?;
        if units.peek().is_none() && eof >= 0 {
            let accept = *d
                .accept
                .get(eof as usize)
                .ok_or("invalid lexer EOF state")?;
            if accept > 0 {
                return d
                    .labels
                    .get((accept - 1) as usize)
                    .map(String::as_str)
                    .ok_or("invalid lexer EOF alternative");
            }
        }
        return Err("no viable lexer alternative");
    }
    Err("lexer prediction work limit exceeded")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imported_tables_match_independent_upstream_prediction_controls() {
        let doc: serde_json::Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/lexer-decision.extract.json"
        ))
        .unwrap();
        for (kerml, language) in [(false, &doc), (true, &doc["kerml"])] {
            let controls = language["prediction_controls"].as_array().unwrap();
            assert!(controls.len() >= 390);
            for control in controls {
                let source = control["source"].as_str().unwrap();
                assert_eq!(
                    predict_in_language(kerml, source).ok(),
                    control["label"].as_str(),
                    "kerml={kerml} {source:?}"
                );
            }
        }
    }
}
