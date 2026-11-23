//! Compare the selector, ordinary fold and tick replay on the same holding.
use crate::{
    reader::{list, members, record, required},
    spec::parsed,
    tick::sha256,
};
use loops::{
    json::Json,
    record::Record,
    result::{result_hash, result_text},
    slice::{Closure, spec_slice},
    spec::Spec,
    spec_hash::Rules,
    things::settle,
    tick::{make_tick, tick_ids},
    tick_check::{TickAnswer, TickSubject, check_tick},
};

fn compare(spec: &Spec, forms: &[Record]) -> Result<Json, String> {
    let whole = settle(forms);
    let expected = spec.run(&whole);
    let canonical = result_text(&expected);
    let hash = result_hash(&sha256, &expected);
    let mut answer = Vec::new();
    for (name, policy) in [("exact", Closure::Exact), ("downward", Closure::Downward)] {
        let ids = spec_slice(spec, &whole, policy);
        let part: Vec<_> = whole
            .iter()
            .filter(|r| ids.contains(&r.id))
            .cloned()
            .collect();
        let result = spec.run(&settle(&part));
        let selected = result_text(&result);
        let selected_hash = result_hash(&sha256, &result);
        if selected != canonical || selected_hash != hash {
            return Err(format!(
                "{name} IDs {}: selected {selected} {selected_hash}, whole {canonical} {hash}",
                tick_ids(&ids).canon()
            ));
        }
        let made = make_tick(spec.clone(), &part);
        let payload = made.to_json(&sha256);
        let subject = TickSubject {
            kind: "tick",
            payload: Some(&payload),
        };
        let check = check_tick(&sha256, &Rules::current(), subject, forms);
        if made.covers != ids
            || result_text(&made.result) != canonical
            || check.answer != TickAnswer::Same
        {
            return Err(format!(
                "{name} IDs {}: tick {}, whole {canonical} {hash}, check {:?}",
                tick_ids(&ids).canon(),
                payload.canon(),
                check.answer
            ));
        }
        answer.push((name.into(), tick_ids(&ids)));
    }
    // The batch evaluator compares this whole answer across input orders.
    answer.push(("canonical".into(), Json::Str(canonical)));
    answer.push(("hash".into(), Json::Str(hash)));
    Ok(Json::Obj(answer))
}

pub(super) fn answers(layer: &str, input: &Json) -> Result<Vec<Json>, String> {
    if layer != "slice" {
        return Err(format!("unknown slice layer {layer}"));
    }
    let kvs = members("input", &["spec", "orders"], input)?;
    let spec = parsed(required("input", kvs, "spec")?)?;
    list("input.orders", required("input", kvs, "orders")?)?
        .iter()
        .enumerate()
        .map(|(i, order)| {
            let path = format!("input.orders[{i}]");
            let forms = list(&path, order)?
                .iter()
                .enumerate()
                .map(|(j, form)| record(&format!("{path}[{j}]"), form))
                .collect::<Result<Vec<_>, _>>()?;
            compare(&spec, &forms).map_err(|e| format!("{path}: {e}"))
        })
        .collect()
}
