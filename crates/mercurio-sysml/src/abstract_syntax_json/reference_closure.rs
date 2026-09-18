//! Structural reference closure for exported standard records. This is not a
//! semantic validator: a resolved ID alone does not establish endpoint legality.
use super::*;

pub(super) fn audit(elements: &[Value]) -> Value {
    let mut ids = BTreeSet::new();
    let mut duplicate_ids = BTreeSet::new();
    for element in elements {
        if let Some(id) = element["@id"].as_str() {
            if !ids.insert(id.to_string()) {
                duplicate_ids.insert(id.to_string());
            }
        }
    }
    let mut unresolved = BTreeMap::<String, Vec<Value>>::new();
    let mut count = 0;
    fn visit(
        value: &Value,
        path: &str,
        source: &str,
        ids: &BTreeSet<String>,
        missing: &mut BTreeMap<String, Vec<Value>>,
        count: &mut usize,
    ) {
        match value {
            Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    visit(item, &format!("{path}[{i}]"), source, ids, missing, count);
                }
            }
            Value::Object(object) => {
                if let Some(target) = object.get("@id").and_then(Value::as_str) {
                    *count += 1;
                    if !ids.contains(target) {
                        missing
                            .entry(target.into())
                            .or_default()
                            .push(json!({"sourceId": source, "property":path}));
                    }
                }
                for (key, item) in object {
                    if key != "xMercurio" {
                        visit(item, &format!("{path}.{key}"), source, ids, missing, count);
                    }
                }
            }
            _ => {}
        }
    }
    for element in elements {
        if let Some(object) = element.as_object() {
            let source = element["@id"].as_str().unwrap_or("<missing-id>");
            for (key, value) in object {
                if key != "xMercurio" {
                    visit(value, key, source, &ids, &mut unresolved, &mut count);
                }
            }
        }
    }
    json!({"closed":unresolved.is_empty() && duplicate_ids.is_empty(),
        "elementCount":elements.len(), "referenceCount":count,
        "duplicateIds":duplicate_ids,
        "unresolved":unresolved.into_iter().map(|(target_id, references)|
            json!({"targetId":target_id,"references":references})).collect::<Vec<_>>()})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_nested_links_and_reports_each_use_without_treating_record_ids_as_links() {
        let report = audit(&[
            json!({"@id":"a","owner":{"@id":"b"},"type":[{"@id":"absent"}],"xMercurio":{"ref":{"@id":"ignored"}}}),
            json!({"@id":"b","target":[{"@id":"absent"}]}),
        ]);
        assert_eq!(report["referenceCount"], 3);
        assert_eq!(report["unresolved"][0]["targetId"], "absent");
        assert_eq!(
            report["unresolved"][0]["references"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            report["unresolved"][0]["references"][0]["property"],
            "type[0]"
        );
        assert_eq!(report["closed"], false);
        assert_eq!(
            audit(&[json!({"@id":"a","owner":{"@id":"a"}})])["closed"],
            true
        );
        assert_eq!(
            audit(&[json!({"@id":"a"}), json!({"@id":"a"})])["duplicateIds"],
            json!(["a"])
        );
    }
}
