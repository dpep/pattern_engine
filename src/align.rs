//! §6.2 — group inputs by skeleton and align them into columns.
//!
//! For the MVP a group is a set of inputs sharing one skeleton; aligning them
//! is just reading off each run position as a column. Multiple skeletons are
//! synthesized independently and alternated by the caller (`synth`).

use crate::tokenize::{Cat, Token, skeleton, tokenize};

/// One aligned column: every value observed at a run position, plus the
/// shared category for that position.
#[derive(Debug, Clone)]
pub struct Column {
    pub cat: Cat,
    pub values: Vec<String>,
}

/// A set of inputs sharing a skeleton, aligned into columns.
#[derive(Debug, Clone)]
pub struct Group {
    pub columns: Vec<Column>,
    /// How many inputs landed in this group (drives alternation ordering).
    pub size: usize,
}

/// Group `inputs` by skeleton. Order is deterministic: by group size desc, then
/// by the skeleton's rendered key lexically.
pub fn align(inputs: &[String]) -> Vec<Group> {
    // Preserve first-seen order of skeletons while collecting members.
    let mut keys: Vec<String> = Vec::new();
    let mut buckets: Vec<(Vec<Vec<Token>>, usize)> = Vec::new();

    for input in inputs {
        let toks = tokenize(input);
        let skel = skeleton(&toks);
        let key = skeleton_key(&skel);
        match keys.iter().position(|k| *k == key) {
            Some(i) => {
                buckets[i].0.push(toks);
                buckets[i].1 += 1;
            }
            None => {
                keys.push(key);
                buckets.push((vec![toks], 1));
            }
        }
    }

    let mut groups: Vec<(String, Group)> = keys
        .into_iter()
        .zip(buckets)
        .map(|(key, (members, size))| (key, to_group(&members, size)))
        .collect();

    groups.sort_by(|a, b| b.1.size.cmp(&a.1.size).then_with(|| a.0.cmp(&b.0)));
    groups.into_iter().map(|(_, g)| g).collect()
}

fn skeleton_key(skel: &[(Cat, Option<char>)]) -> String {
    let mut s = String::new();
    for (cat, lit) in skel {
        let tag = match cat {
            Cat::Digit => 'D',
            Cat::Lower => 'l',
            Cat::Upper => 'U',
            Cat::Space => 's',
            Cat::Literal => 'L',
        };
        s.push(tag);
        if let Some(c) = lit {
            s.push(*c);
        }
        s.push('|');
    }
    s
}

fn to_group(members: &[Vec<Token>], size: usize) -> Group {
    let ncols = members.first().map_or(0, |m| m.len());
    let mut columns = Vec::with_capacity(ncols);
    for i in 0..ncols {
        let cat = members[0][i].cat;
        let values = members.iter().map(|m| m[i].text.clone()).collect();
        columns.push(Column { cat, values });
    }
    Group { columns, size }
}
