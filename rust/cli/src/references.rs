//! Reference coverage uses raw live payload values, including repeated list members.
use loops::{envelope::typed_ref, json::Json};

#[derive(Default)]
pub(crate) struct Coverage {
    pub(crate) not_typed: usize,
    pub(crate) not_read: usize,
}

/// Count immediate list members separately. An unopened setter has unknown
/// cardinality and contributes just one unread occurrence.
pub(crate) fn count_values<'a>(
    setters: impl Iterator<Item = Option<&'a Json>>,
    mut typed: impl FnMut(&'a str),
) -> Coverage {
    let mut counts = Coverage::default();
    for setter in setters {
        let values = match setter {
            None => {
                counts.not_read += 1;
                continue;
            }
            Some(Json::Arr(items)) => items.as_slice(),
            Some(single) => std::slice::from_ref(single),
        };
        for value in values {
            match value {
                Json::Str(s) if typed_ref(s) => typed(s),
                _ => counts.not_typed += 1,
            }
        }
    }
    counts
}
