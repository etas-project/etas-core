//! Canonical wire order for unordered compiler-data containers.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{
    Serialize, Serializer,
    ser::{SerializeMap, SerializeSeq},
};

pub fn map<S: Serializer, K: Serialize + Ord, V: Serialize>(
    value: &HashMap<K, V>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut entries = value.iter().collect::<Vec<_>>();
    entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
    let mut output = serializer.serialize_map(Some(entries.len()))?;
    for (key, value) in entries {
        output.serialize_entry(key, value)?;
    }
    output.end()
}

pub fn set<S: Serializer, T: Serialize + Ord>(
    value: &HashSet<T>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut entries = value.iter().collect::<Vec<_>>();
    entries.sort_unstable();
    let mut output = serializer.serialize_seq(Some(entries.len()))?;
    for value in entries {
        output.serialize_element(value)?;
    }
    output.end()
}

pub fn nested_map<S: Serializer, K: Serialize + Ord, L: Serialize + Ord, V: Serialize>(
    value: &HashMap<K, HashMap<L, V>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    struct Ordered<'a, K, V>(&'a HashMap<K, V>);
    impl<K: Serialize + Ord, V: Serialize> Serialize for Ordered<'_, K, V> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            map(self.0, serializer)
        }
    }
    value
        .iter()
        .map(|(key, inner)| (key, Ordered(inner)))
        .collect::<BTreeMap<_, _>>()
        .serialize(serializer)
}
