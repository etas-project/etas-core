use std::{collections::HashMap, sync::Arc};

use super::*;

#[derive(crate::RetainedSize)]
struct Shared {
    first: Arc<Vec<String>>,
    second: Arc<Vec<String>>,
}

#[test]
fn shared_allocations_are_counted_once_and_capacity_not_length_is_charged() {
    let mut text = String::with_capacity(4096);
    text.push('x');
    let values = Arc::new(vec![text]);
    let one = MemoryFootprint::of(&values);
    let both = MemoryFootprint::of(&Shared {
        first: values.clone(),
        second: values,
    });
    assert_eq!(one, both);
    assert!(one.estimated_bytes() >= 4096);
    assert!(one.estimated_bytes() < 8192);
}

#[test]
fn reserved_empty_containers_are_not_free() {
    let vector = Vec::<String>::with_capacity(100);
    let map = HashMap::<String, String>::with_capacity(100);
    assert_eq!(
        MemoryFootprint::of(&vector).estimated_bytes(),
        (100 * size_of::<String>()) as u64
    );
    assert!(
        MemoryFootprint::of(&map).estimated_bytes() > 100 * size_of::<(String, String)>() as u64
    );
    assert_eq!(
        MemoryFootprint::of(&Vec::<String>::new()).estimated_bytes(),
        0
    );
}

#[derive(crate::RetainedSize)]
enum Node {
    End(String),
    Next(Box<Node>),
}

#[test]
fn derive_traverses_deep_graphs_without_using_the_call_stack() {
    let mut node = Node::End("owned".to_owned());
    for _ in 0..10_000 {
        node = Node::Next(Box::new(node));
    }
    let footprint = MemoryFootprint::of(&node);
    assert_eq!(footprint.allocations.len(), 10_001);
    while let Node::Next(next) = node {
        node = *next;
    }
}

#[test]
fn borrowed_data_is_not_charged_as_owned() {
    let text = String::with_capacity(4096);
    assert_eq!(MemoryFootprint::of(&(&text, "static")).estimated_bytes(), 0);
}

#[derive(crate::RetainedSize)]
struct Generic<'__etas_memory0, T, const N: usize> {
    owned: Vec<T>,
    borrowed: &'__etas_memory0 [u8],
    inline: [u8; N],
}

#[test]
fn derive_supports_type_const_and_lifetime_parameters_without_hiding_owned_fields() {
    let borrowed = vec![0; 100_000];
    let value = Generic {
        owned: vec![String::with_capacity(100)],
        borrowed: &borrowed,
        inline: [0; 3],
    };
    let footprint = MemoryFootprint::of(&value);
    assert!(footprint.estimated_bytes() >= 100);
    assert!(footprint.estimated_bytes() < 1000);
}

#[test]
fn inline_elements_do_not_need_an_element_worklist_but_the_buffer_is_charged() {
    #[derive(crate::RetainedSize)]
    struct Inline {
        id: u32,
        flags: [bool; 4],
    }
    assert!(!Inline::may_retain_heap());
    assert!(!Option::<Inline>::may_retain_heap());
    assert!(!<(u32, Inline)>::may_retain_heap());
    assert!(Vec::<Inline>::may_retain_heap());
    let values = vec![0_u8; 1_000_000];
    let footprint = MemoryFootprint::of(&values);
    assert_eq!(footprint.allocations.len(), 1);
    assert_eq!(footprint.estimated_bytes(), values.capacity() as u64);
}
