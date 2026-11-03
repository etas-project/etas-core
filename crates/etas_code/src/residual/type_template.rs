//! Bounded, source-independent structural recipes for symbolic type bindings.
//! These values describe substitutions; they are not checked-plan evidence.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    CodeFingerprint,
    descriptor::{
        BuiltinTypeInfo, DescriptorError, DescriptorGraph, DescriptorGraphInput, DescriptorLimits,
        FieldInfo, PrimitiveInfo, TypeInfo, TypeInfoRef, builtin_tag, primitive_tag,
    },
    identity::canonical::CanonicalEncoder,
    residual::identity::GenericParameterId,
};

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct TemplateField {
    #[serde(deserialize_with = "deserialize_bounded_name")]
    pub name: String,
    pub ty: u32,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub enum StaticTypeNode {
    Primitive(PrimitiveInfo),
    Parameter(GenericParameterId),
    Builtin {
        constructor: BuiltinTypeInfo,
        #[serde(deserialize_with = "deserialize_bounded_edges")]
        arguments: Vec<u32>,
    },
    Tuple(#[serde(deserialize_with = "deserialize_bounded_edges")] Vec<u32>),
    Record(#[serde(deserialize_with = "deserialize_bounded_fields")] Vec<TemplateField>),
}

impl StaticTypeNode {
    fn child_count(&self) -> usize {
        match self {
            Self::Primitive(_) | Self::Parameter(_) => 0,
            Self::Builtin { arguments, .. } | Self::Tuple(arguments) => arguments.len(),
            Self::Record(fields) => fields.len(),
        }
    }

    fn children(&self) -> Vec<u32> {
        match self {
            Self::Primitive(_) | Self::Parameter(_) => vec![],
            Self::Builtin { arguments, .. } | Self::Tuple(arguments) => arguments.clone(),
            Self::Record(fields) => fields.iter().map(|field| field.ty).collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeTemplateLimits {
    pub max_nodes: usize,
    pub max_edges: usize,
    pub max_depth: usize,
    pub max_text_bytes: usize,
    pub max_work: usize,
}

impl Default for TypeTemplateLimits {
    fn default() -> Self {
        Self {
            max_nodes: 1024,
            max_edges: 4096,
            max_depth: 64,
            max_text_bytes: 65_536,
            max_work: 1_048_576,
        }
    }
}

impl TypeTemplateLimits {
    fn capped(self) -> Self {
        let absolute = Self::default();
        Self {
            max_nodes: self.max_nodes.min(absolute.max_nodes),
            max_edges: self.max_edges.min(absolute.max_edges),
            max_depth: self.max_depth.min(absolute.max_depth),
            max_text_bytes: self.max_text_bytes.min(absolute.max_text_bytes),
            max_work: self.max_work.min(absolute.max_work),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeTemplateError {
    Empty,
    NodeLimit,
    EdgeLimit,
    DepthLimit,
    TextLimit,
    WorkLimit,
    InvalidRoot,
    InvalidReference,
    InvalidArity,
    InvalidFieldName,
    DuplicateFieldName,
    UnreachableNode,
    Descriptor(DescriptorError),
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, etas_core::RetainedSize)]
pub struct StaticTypeTemplate {
    nodes: Vec<StaticTypeNode>,
    root: u32,
}

impl<'de> serde::Deserialize<'de> for StaticTypeTemplate {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Raw {
            #[serde(deserialize_with = "deserialize_bounded_nodes")]
            nodes: Vec<StaticTypeNode>,
            root: u32,
        }
        let raw = Raw::deserialize(deserializer)?;
        Self::new(raw.nodes, raw.root, TypeTemplateLimits::default()).map_err(|error| {
            serde::de::Error::custom(format!("invalid static type template: {error:?}"))
        })
    }
}

fn deserialize_bounded_name<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = String;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a bounded field name")
        }
        fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<String, E> {
            if value.len() > TypeTemplateLimits::default().max_text_bytes {
                return Err(E::custom("type template field name exceeds limit"));
            }
            Ok(value.to_owned())
        }
        fn visit_string<E: serde::de::Error>(self, value: String) -> Result<String, E> {
            if value.len() > TypeTemplateLimits::default().max_text_bytes {
                return Err(E::custom("type template field name exceeds limit"));
            }
            Ok(value)
        }
    }
    deserializer.deserialize_string(Visitor)
}

fn deserialize_bounded_edges<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<u32>, D::Error> {
    deserialize_bounded_vec(deserializer, TypeTemplateLimits::default().max_edges)
}

fn deserialize_bounded_fields<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<TemplateField>, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Vec<TemplateField>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("bounded template fields")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            use serde::de::Error;
            let limits = TypeTemplateLimits::default();
            if seq.size_hint().is_some_and(|len| len > limits.max_edges) {
                return Err(A::Error::custom("template field limit exceeded"));
            }
            let mut fields = Vec::new();
            let mut text = 0usize;
            while let Some(field) = seq.next_element::<TemplateField>()? {
                text = text
                    .checked_add(field.name.len())
                    .ok_or_else(|| A::Error::custom("template text limit exceeded"))?;
                if fields.len() >= limits.max_edges || text > limits.max_text_bytes {
                    return Err(A::Error::custom("template field limit exceeded"));
                }
                fields.push(field);
            }
            Ok(fields)
        }
    }
    deserializer.deserialize_seq(Visitor)
}

fn deserialize_bounded_nodes<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<StaticTypeNode>, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Vec<StaticTypeNode>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("bounded template nodes")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            use serde::de::Error;
            let limits = TypeTemplateLimits::default();
            if seq.size_hint().is_some_and(|len| len > limits.max_nodes) {
                return Err(A::Error::custom("template node limit exceeded"));
            }
            let mut nodes = Vec::new();
            let mut edges = 0usize;
            let mut text = 0usize;
            while let Some(node) = seq.next_element::<StaticTypeNode>()? {
                edges = edges
                    .checked_add(node.children().len())
                    .ok_or_else(|| A::Error::custom("template edge limit exceeded"))?;
                if let StaticTypeNode::Record(fields) = &node {
                    for field in fields {
                        text = text
                            .checked_add(field.name.len())
                            .ok_or_else(|| A::Error::custom("template text limit exceeded"))?;
                    }
                }
                if nodes.len() >= limits.max_nodes
                    || edges > limits.max_edges
                    || text > limits.max_text_bytes
                {
                    return Err(A::Error::custom("template graph limit exceeded"));
                }
                nodes.push(node);
            }
            Ok(nodes)
        }
    }
    deserializer.deserialize_seq(Visitor)
}

fn deserialize_bounded_vec<'de, D, T>(deserializer: D, max: usize) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    struct Visitor<T> {
        max: usize,
        marker: std::marker::PhantomData<T>,
    }
    impl<'de, T: serde::Deserialize<'de>> serde::de::Visitor<'de> for Visitor<T> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a bounded sequence")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            use serde::de::Error;
            if seq.size_hint().is_some_and(|len| len > self.max) {
                return Err(A::Error::custom("template sequence limit exceeded"));
            }
            let mut values = Vec::new();
            while let Some(value) = seq.next_element()? {
                if values.len() >= self.max {
                    return Err(A::Error::custom("template sequence limit exceeded"));
                }
                values.push(value);
            }
            Ok(values)
        }
    }
    deserializer.deserialize_seq(Visitor {
        max,
        marker: std::marker::PhantomData,
    })
}

impl StaticTypeTemplate {
    /// Nodes are in dependency order; every child must precede its parent.
    pub fn new(
        nodes: Vec<StaticTypeNode>,
        root: u32,
        limits: TypeTemplateLimits,
    ) -> Result<Self, TypeTemplateError> {
        Self::new_with_work(nodes, root, limits, &mut 0)
    }

    fn new_with_work(
        nodes: Vec<StaticTypeNode>,
        root: u32,
        limits: TypeTemplateLimits,
        work: &mut usize,
    ) -> Result<Self, TypeTemplateError> {
        let limits = limits.capped();
        if nodes.is_empty() {
            return Err(TypeTemplateError::Empty);
        }
        if nodes.len() > limits.max_nodes {
            return Err(TypeTemplateError::NodeLimit);
        }
        if root as usize >= nodes.len() {
            return Err(TypeTemplateError::InvalidRoot);
        }
        let mut edges = 0usize;
        let mut text = 0usize;
        let mut depth = Vec::with_capacity(nodes.len());
        for (index, node) in nodes.iter().enumerate() {
            if let StaticTypeNode::Builtin {
                constructor,
                arguments,
            } = node
                && constructor.arity() != arguments.len()
            {
                return Err(TypeTemplateError::InvalidArity);
            }
            edges = edges
                .checked_add(node.child_count())
                .ok_or(TypeTemplateError::EdgeLimit)?;
            if edges > limits.max_edges {
                return Err(TypeTemplateError::EdgeLimit);
            }
            if let StaticTypeNode::Record(fields) = node {
                let mut names = BTreeSet::new();
                for field in fields {
                    text = text
                        .checked_add(field.name.len())
                        .ok_or(TypeTemplateError::TextLimit)?;
                    if text > limits.max_text_bytes {
                        return Err(TypeTemplateError::TextLimit);
                    }
                    if field.name.is_empty() || field.name.contains('\0') {
                        return Err(TypeTemplateError::InvalidFieldName);
                    }
                    if !names.insert(field.name.as_str()) {
                        return Err(TypeTemplateError::DuplicateFieldName);
                    }
                }
            }
            let mut height = 1usize;
            for child in node.children() {
                if child as usize >= index {
                    return Err(TypeTemplateError::InvalidReference);
                }
                height = height.max(depth[child as usize] + 1);
            }
            if height > limits.max_depth {
                return Err(TypeTemplateError::DepthLimit);
            }
            depth.push(height);
        }
        let mut visited = BTreeSet::new();
        let mut pending = vec![root];
        while let Some(index) = pending.pop() {
            if visited.insert(index) {
                pending.extend(nodes[index as usize].children());
            }
        }
        if visited.len() != nodes.len() {
            return Err(TypeTemplateError::UnreachableNode);
        }
        let mut normalized = Vec::with_capacity(nodes.len());
        let mut indices = Vec::with_capacity(nodes.len());
        for node in &nodes {
            let node = remap_node(node, &indices)?;
            indices.push(intern_node(&mut normalized, node, work, limits)?);
        }
        let root = indices[root as usize];
        let mut canonical = Vec::with_capacity(normalized.len());
        let mut mapped = vec![u32::MAX; normalized.len()];
        let mut pending = vec![(root, false)];
        while let Some((index, expanded)) = pending.pop() {
            *work = work.checked_add(1).ok_or(TypeTemplateError::WorkLimit)?;
            if *work > limits.max_work {
                return Err(TypeTemplateError::WorkLimit);
            }
            if mapped[index as usize] != u32::MAX {
                continue;
            }
            let node = &normalized[index as usize];
            if !expanded {
                pending.push((index, true));
                pending.extend(
                    node.children()
                        .into_iter()
                        .rev()
                        .map(|child| (child, false)),
                );
                continue;
            }
            for child in node.children() {
                if mapped[child as usize] == u32::MAX {
                    return Err(TypeTemplateError::InvalidReference);
                }
            }
            let canonical_index =
                u32::try_from(canonical.len()).map_err(|_| TypeTemplateError::NodeLimit)?;
            canonical.push(remap_node(node, &mapped)?);
            mapped[index as usize] = canonical_index;
        }
        Ok(Self {
            nodes: canonical,
            root: mapped[root as usize],
        })
    }

    pub fn nodes(&self) -> &[StaticTypeNode] {
        &self.nodes
    }
    pub fn root(&self) -> u32 {
        self.root
    }

    pub fn resource_usage(&self) -> (usize, usize, usize) {
        let mut edges = 0usize;
        let mut text = 0usize;
        for node in &self.nodes {
            match node {
                StaticTypeNode::Builtin { arguments, .. } | StaticTypeNode::Tuple(arguments) => {
                    edges += arguments.len()
                }
                StaticTypeNode::Record(fields) => {
                    edges += fields.len();
                    text += fields.iter().map(|field| field.name.len()).sum::<usize>();
                }
                _ => {}
            }
        }
        (self.nodes.len(), edges, text)
    }

    /// Canonical traversal labels do not include node indices from the producer.
    pub fn fingerprint(&self) -> CodeFingerprint {
        let mut encoder = CanonicalEncoder::new("etas.code.residual.static-type-template.v1");
        let mut seen = BTreeMap::new();
        let mut work = vec![self.root];
        while let Some(index) = work.pop() {
            if let Some(label) = seen.get(&index) {
                encoder.tag(0);
                encoder.number(*label);
                continue;
            }
            let label = seen.len() as u64;
            seen.insert(index, label);
            encoder.tag(1);
            encoder.number(label);
            let node = &self.nodes[index as usize];
            match node {
                StaticTypeNode::Primitive(value) => {
                    encoder.tag(1);
                    encoder.tag(primitive_tag(*value));
                }
                StaticTypeNode::Parameter(value) => {
                    encoder.tag(2);
                    encoder.fingerprint(value.fingerprint());
                }
                StaticTypeNode::Builtin { constructor, .. } => {
                    encoder.tag(3);
                    encoder.tag(builtin_tag(*constructor));
                }
                StaticTypeNode::Tuple(_) => encoder.tag(4),
                StaticTypeNode::Record(fields) => {
                    encoder.tag(5);
                    for field in fields {
                        encoder.bytes(field.name.as_bytes());
                    }
                }
            }
            let children = node.children();
            encoder.number(children.len() as u64);
            work.extend(children.into_iter().rev());
        }
        encoder.finish()
    }

    /// Simultaneously substitutes the supplied binders once. Replacement
    /// templates may themselves remain symbolic and are not recursively solved.
    pub fn substitute(
        &self,
        bindings: &BTreeMap<GenericParameterId, StaticTypeTemplate>,
        limits: TypeTemplateLimits,
    ) -> Result<Self, TypeTemplateError> {
        let limits = limits.capped();
        if self.nodes.len() > limits.max_nodes {
            return Err(TypeTemplateError::NodeLimit);
        }
        let mut output = Vec::new();
        let mut indices = Vec::with_capacity(self.nodes.len());
        let mut replacements = BTreeMap::new();
        let mut work = 0usize;
        for node in &self.nodes {
            let index = if let StaticTypeNode::Parameter(parameter) = node
                && let Some(template) = bindings.get(parameter)
            {
                if let Some(index) = replacements.get(parameter) {
                    *index
                } else {
                    if template.nodes.len() > limits.max_nodes {
                        return Err(TypeTemplateError::NodeLimit);
                    }
                    let mut mapped = Vec::with_capacity(template.nodes.len());
                    for node in &template.nodes {
                        let node = remap_node(node, &mapped)?;
                        mapped.push(intern_node(&mut output, node, &mut work, limits)?);
                    }
                    let index = *mapped
                        .get(template.root as usize)
                        .ok_or(TypeTemplateError::InvalidRoot)?;
                    replacements.insert(*parameter, index);
                    index
                }
            } else {
                intern_node(&mut output, remap_node(node, &indices)?, &mut work, limits)?
            };
            indices.push(index);
        }
        let root = *indices
            .get(self.root as usize)
            .ok_or(TypeTemplateError::InvalidRoot)?;
        Self::new_with_work(output, root, limits, &mut work)
    }

    /// A closed recipe uses exactly the existing descriptor-graph fingerprint.
    pub fn closed_descriptor_fingerprint(
        &self,
        limits: DescriptorLimits,
    ) -> Result<Option<CodeFingerprint>, TypeTemplateError> {
        let mut types = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            types.push(match node {
                StaticTypeNode::Primitive(value) => TypeInfo::Primitive(*value),
                StaticTypeNode::Parameter(_) => return Ok(None),
                StaticTypeNode::Builtin {
                    constructor,
                    arguments,
                } => TypeInfo::Builtin {
                    constructor: *constructor,
                    arguments: arguments.iter().copied().map(TypeInfoRef).collect(),
                },
                StaticTypeNode::Tuple(arguments) => {
                    TypeInfo::Tuple(arguments.iter().copied().map(TypeInfoRef).collect())
                }
                StaticTypeNode::Record(fields) => TypeInfo::Record(
                    fields
                        .iter()
                        .map(|field| FieldInfo {
                            name: field.name.clone(),
                            ty: TypeInfoRef(field.ty),
                        })
                        .collect(),
                ),
            });
        }
        let graph = DescriptorGraph::new(
            DescriptorGraphInput {
                types,
                ..Default::default()
            },
            limits,
        )
        .map_err(TypeTemplateError::Descriptor)?;
        graph
            .type_fingerprint(TypeInfoRef(self.root))
            .map(Some)
            .map_err(TypeTemplateError::Descriptor)
    }
}

fn remap_node(node: &StaticTypeNode, indices: &[u32]) -> Result<StaticTypeNode, TypeTemplateError> {
    let map = |index: &u32| {
        indices
            .get(*index as usize)
            .copied()
            .ok_or(TypeTemplateError::InvalidReference)
    };
    Ok(match node {
        StaticTypeNode::Primitive(value) => StaticTypeNode::Primitive(*value),
        StaticTypeNode::Parameter(value) => StaticTypeNode::Parameter(*value),
        StaticTypeNode::Builtin {
            constructor,
            arguments,
        } => StaticTypeNode::Builtin {
            constructor: *constructor,
            arguments: arguments.iter().map(map).collect::<Result<_, _>>()?,
        },
        StaticTypeNode::Tuple(arguments) => {
            StaticTypeNode::Tuple(arguments.iter().map(map).collect::<Result<_, _>>()?)
        }
        StaticTypeNode::Record(fields) => StaticTypeNode::Record(
            fields
                .iter()
                .map(|field| {
                    Ok(TemplateField {
                        name: field.name.clone(),
                        ty: map(&field.ty)?,
                    })
                })
                .collect::<Result<_, TypeTemplateError>>()?,
        ),
    })
}

fn intern_node(
    output: &mut Vec<StaticTypeNode>,
    node: StaticTypeNode,
    work: &mut usize,
    limits: TypeTemplateLimits,
) -> Result<u32, TypeTemplateError> {
    let comparison_cost = match &node {
        StaticTypeNode::Primitive(_) | StaticTypeNode::Parameter(_) => Some(1usize),
        StaticTypeNode::Builtin { arguments, .. } | StaticTypeNode::Tuple(arguments) => {
            arguments.len().checked_add(1)
        }
        StaticTypeNode::Record(fields) => fields.iter().try_fold(1usize, |cost, field| {
            cost.checked_add(1)?.checked_add(field.name.len())
        }),
    }
    .ok_or(TypeTemplateError::WorkLimit)?;
    *work = work
        .checked_add(comparison_cost)
        .ok_or(TypeTemplateError::WorkLimit)?;
    if *work > limits.max_work {
        return Err(TypeTemplateError::WorkLimit);
    }
    for (index, existing) in output.iter().enumerate() {
        *work = work
            .checked_add(comparison_cost)
            .ok_or(TypeTemplateError::WorkLimit)?;
        if *work > limits.max_work {
            return Err(TypeTemplateError::WorkLimit);
        }
        if existing == &node {
            return u32::try_from(index).map_err(|_| TypeTemplateError::NodeLimit);
        }
    }
    if output.len() >= limits.max_nodes {
        return Err(TypeTemplateError::NodeLimit);
    }
    let index = u32::try_from(output.len()).map_err(|_| TypeTemplateError::NodeLimit)?;
    output.push(node);
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> TypeTemplateLimits {
        TypeTemplateLimits {
            max_nodes: 32,
            max_edges: 64,
            max_depth: 16,
            max_text_bytes: 128,
            max_work: 1024,
        }
    }
    fn parameter(byte: u8) -> GenericParameterId {
        GenericParameterId::from_canonical_fingerprint(CodeFingerprint::from_bytes([byte; 32]))
    }

    #[test]
    fn substitution_preserves_structure_and_matches_closed_descriptor() {
        let binder = parameter(1);
        let source = StaticTypeTemplate::new(
            vec![
                StaticTypeNode::Parameter(binder),
                StaticTypeNode::Builtin {
                    constructor: BuiltinTypeInfo::Array,
                    arguments: vec![0],
                },
            ],
            1,
            limits(),
        )
        .unwrap();
        assert!(
            source
                .closed_descriptor_fingerprint(DescriptorLimits::default())
                .unwrap()
                .is_none()
        );
        let integer = StaticTypeTemplate::new(
            vec![StaticTypeNode::Primitive(PrimitiveInfo::I32)],
            0,
            limits(),
        )
        .unwrap();
        let resolved = source
            .substitute(&[(binder, integer)].into_iter().collect(), limits())
            .unwrap();
        let graph = DescriptorGraph::new(
            DescriptorGraphInput {
                types: vec![
                    TypeInfo::Primitive(PrimitiveInfo::I32),
                    TypeInfo::Builtin {
                        constructor: BuiltinTypeInfo::Array,
                        arguments: vec![TypeInfoRef(0)],
                    },
                ],
                ..Default::default()
            },
            DescriptorLimits::default(),
        )
        .unwrap();
        assert_eq!(
            resolved
                .closed_descriptor_fingerprint(DescriptorLimits::default())
                .unwrap(),
            Some(graph.type_fingerprint(TypeInfoRef(1)).unwrap())
        );
        assert_ne!(resolved.fingerprint(), source.fingerprint());
        assert_eq!(
            source.substitute(&BTreeMap::new(), limits()).unwrap(),
            source
        );
    }

    #[test]
    fn fingerprint_is_independent_of_topological_node_numbering() {
        let first = StaticTypeTemplate::new(
            vec![
                StaticTypeNode::Primitive(PrimitiveInfo::I32),
                StaticTypeNode::Primitive(PrimitiveInfo::Bool),
                StaticTypeNode::Builtin {
                    constructor: BuiltinTypeInfo::Map,
                    arguments: vec![0, 1],
                },
            ],
            2,
            limits(),
        )
        .unwrap();
        let second = StaticTypeTemplate::new(
            vec![
                StaticTypeNode::Primitive(PrimitiveInfo::Bool),
                StaticTypeNode::Primitive(PrimitiveInfo::I32),
                StaticTypeNode::Builtin {
                    constructor: BuiltinTypeInfo::Map,
                    arguments: vec![1, 0],
                },
            ],
            2,
            limits(),
        )
        .unwrap();
        assert_eq!(first.fingerprint(), second.fingerprint());
        assert_eq!(first, second);
        assert_eq!(
            first
                .closed_descriptor_fingerprint(DescriptorLimits::default())
                .unwrap(),
            second
                .closed_descriptor_fingerprint(DescriptorLimits::default())
                .unwrap()
        );
    }

    #[test]
    fn fingerprint_is_independent_of_equivalent_node_sharing() {
        let binder = parameter(1);
        let shared = StaticTypeTemplate::new(
            vec![
                StaticTypeNode::Parameter(binder),
                StaticTypeNode::Tuple(vec![0, 0]),
            ],
            1,
            limits(),
        )
        .unwrap();
        let duplicated = StaticTypeTemplate::new(
            vec![
                StaticTypeNode::Parameter(binder),
                StaticTypeNode::Parameter(binder),
                StaticTypeNode::Tuple(vec![0, 1]),
            ],
            2,
            limits(),
        )
        .unwrap();
        assert_eq!(shared, duplicated);
        assert_eq!(shared.fingerprint(), duplicated.fingerprint());
    }

    #[test]
    fn work_limit_accounts_for_field_names_and_edges() {
        let fields = vec![TemplateField {
            name: "a".repeat(64),
            ty: 0,
        }];
        let graph = vec![
            StaticTypeNode::Primitive(PrimitiveInfo::I32),
            StaticTypeNode::Record(fields),
        ];
        assert!(matches!(
            StaticTypeTemplate::new(
                graph.clone(),
                1,
                TypeTemplateLimits {
                    max_work: 100,
                    ..limits()
                },
            ),
            Err(TypeTemplateError::WorkLimit)
        ));
        assert!(StaticTypeTemplate::new(graph, 1, limits()).is_ok());
    }

    #[test]
    fn symbolic_records_reject_invalid_or_duplicate_field_names() {
        let make = |names: &[&str]| {
            vec![
                StaticTypeNode::Parameter(parameter(1)),
                StaticTypeNode::Record(
                    names
                        .iter()
                        .map(|name| TemplateField {
                            name: (*name).into(),
                            ty: 0,
                        })
                        .collect(),
                ),
            ]
        };
        for name in ["", "bad\0name"] {
            assert!(matches!(
                StaticTypeTemplate::new(make(&[name]), 1, limits()),
                Err(TypeTemplateError::InvalidFieldName)
            ));
        }
        assert!(matches!(
            StaticTypeTemplate::new(make(&["value", "value"]), 1, limits()),
            Err(TypeTemplateError::DuplicateFieldName)
        ));
        let encoded = serde_json::json!({
            "nodes": make(&["value", "value"]),
            "root": 1,
        });
        assert!(serde_json::from_value::<StaticTypeTemplate>(encoded).is_err());
        assert!(StaticTypeTemplate::new(make(&["left", "right"]), 1, limits()).is_ok());
    }

    #[test]
    fn substitution_and_normalization_share_one_work_budget() {
        let graph = vec![
            StaticTypeNode::Parameter(parameter(1)),
            StaticTypeNode::Builtin {
                constructor: BuiltinTypeInfo::Array,
                arguments: vec![0],
            },
        ];
        let bounded = TypeTemplateLimits {
            max_work: 9,
            ..limits()
        };
        let template = StaticTypeTemplate::new(graph, 1, bounded).unwrap();
        assert!(matches!(
            template.substitute(&BTreeMap::new(), bounded),
            Err(TypeTemplateError::WorkLimit)
        ));
        assert_eq!(
            template
                .substitute(
                    &BTreeMap::new(),
                    TypeTemplateLimits {
                        max_work: 14,
                        ..limits()
                    },
                )
                .unwrap(),
            template
        );
    }

    #[test]
    fn invalid_or_excessive_templates_are_rejected() {
        assert!(matches!(
            StaticTypeTemplate::new(
                vec![
                    StaticTypeNode::Primitive(PrimitiveInfo::I32),
                    StaticTypeNode::Tuple(vec![0; 65]),
                ],
                1,
                limits(),
            ),
            Err(TypeTemplateError::EdgeLimit)
        ));
        assert!(matches!(
            StaticTypeTemplate::new(vec![StaticTypeNode::Tuple(vec![0])], 0, limits()),
            Err(TypeTemplateError::InvalidReference)
        ));
        assert!(matches!(
            StaticTypeTemplate::new(
                vec![
                    StaticTypeNode::Primitive(PrimitiveInfo::I32),
                    StaticTypeNode::Primitive(PrimitiveInfo::Bool)
                ],
                0,
                limits()
            ),
            Err(TypeTemplateError::UnreachableNode)
        ));
        assert!(matches!(
            StaticTypeTemplate::new(
                vec![StaticTypeNode::Builtin {
                    constructor: BuiltinTypeInfo::Array,
                    arguments: vec![]
                }],
                0,
                limits()
            ),
            Err(TypeTemplateError::InvalidArity)
        ));
        let node = vec![
            StaticTypeNode::Primitive(PrimitiveInfo::I32),
            StaticTypeNode::Tuple(vec![0]),
        ];
        assert!(matches!(
            StaticTypeTemplate::new(
                node.clone(),
                1,
                TypeTemplateLimits {
                    max_nodes: 1,
                    ..limits()
                }
            ),
            Err(TypeTemplateError::NodeLimit)
        ));
        assert!(matches!(
            StaticTypeTemplate::new(
                node.clone(),
                1,
                TypeTemplateLimits {
                    max_edges: 0,
                    ..limits()
                }
            ),
            Err(TypeTemplateError::EdgeLimit)
        ));
        assert!(matches!(
            StaticTypeTemplate::new(
                node,
                1,
                TypeTemplateLimits {
                    max_depth: 1,
                    ..limits()
                }
            ),
            Err(TypeTemplateError::DepthLimit)
        ));
        let parameterized = StaticTypeTemplate::new(
            vec![
                StaticTypeNode::Parameter(parameter(1)),
                StaticTypeNode::Tuple(vec![0, 0]),
            ],
            1,
            limits(),
        )
        .unwrap();
        assert!(matches!(
            parameterized.substitute(
                &BTreeMap::new(),
                TypeTemplateLimits {
                    max_work: 0,
                    ..limits()
                }
            ),
            Err(TypeTemplateError::WorkLimit)
        ));
    }

    #[test]
    fn substitutions_are_simultaneous_and_preserve_symbolic_replacements() {
        let first = parameter(1);
        let second = parameter(2);
        let source =
            StaticTypeTemplate::new(vec![StaticTypeNode::Parameter(first)], 0, limits()).unwrap();
        let symbolic =
            StaticTypeTemplate::new(vec![StaticTypeNode::Parameter(second)], 0, limits()).unwrap();
        let integer = StaticTypeTemplate::new(
            vec![StaticTypeNode::Primitive(PrimitiveInfo::I32)],
            0,
            limits(),
        )
        .unwrap();
        let bindings = [(first, symbolic.clone()), (second, integer.clone())]
            .into_iter()
            .collect();
        let once = source.substitute(&bindings, limits()).unwrap();
        assert_eq!(once, symbolic);
        assert!(
            once.closed_descriptor_fingerprint(DescriptorLimits::default())
                .unwrap()
                .is_none()
        );
        assert_eq!(once.substitute(&bindings, limits()).unwrap(), integer);
    }

    #[test]
    fn serialized_templates_validate_graph_and_limits_on_decode() {
        let source = StaticTypeTemplate::new(
            vec![
                StaticTypeNode::Parameter(parameter(1)),
                StaticTypeNode::Builtin {
                    constructor: BuiltinTypeInfo::Array,
                    arguments: vec![0],
                },
            ],
            1,
            limits(),
        )
        .unwrap();
        let decoded: StaticTypeTemplate =
            serde_json::from_str(&serde_json::to_string(&source).unwrap()).unwrap();
        assert_eq!(decoded, source);
        let mut cyclic = serde_json::to_value(&source).unwrap();
        cyclic["nodes"][1]["Builtin"]["arguments"][0] = serde_json::json!(1);
        assert!(serde_json::from_value::<StaticTypeTemplate>(cyclic).is_err());
        let mut oversized = serde_json::to_value(&source).unwrap();
        oversized["nodes"] = serde_json::json!(vec![
            serde_json::json!({"Primitive":"I32"});
            TypeTemplateLimits::default().max_nodes + 1
        ]);
        assert!(serde_json::from_value::<StaticTypeTemplate>(oversized).is_err());
    }

    #[test]
    fn record_field_names_and_substitutions_contribute_to_identity() {
        let binder = parameter(1);
        let record = |name: &str| {
            StaticTypeTemplate::new(
                vec![
                    StaticTypeNode::Parameter(binder),
                    StaticTypeNode::Record(vec![TemplateField {
                        name: name.into(),
                        ty: 0,
                    }]),
                ],
                1,
                limits(),
            )
            .unwrap()
        };
        assert_ne!(record("left").fingerprint(), record("right").fingerprint());
        let integer = StaticTypeTemplate::new(
            vec![StaticTypeNode::Primitive(PrimitiveInfo::I32)],
            0,
            limits(),
        )
        .unwrap();
        let resolved = record("left")
            .substitute(&[(binder, integer)].into_iter().collect(), limits())
            .unwrap();
        let graph = DescriptorGraph::new(
            DescriptorGraphInput {
                types: vec![
                    TypeInfo::Primitive(PrimitiveInfo::I32),
                    TypeInfo::Record(vec![FieldInfo {
                        name: "left".into(),
                        ty: TypeInfoRef(0),
                    }]),
                ],
                ..Default::default()
            },
            DescriptorLimits::default(),
        )
        .unwrap();
        assert_eq!(
            resolved
                .closed_descriptor_fingerprint(DescriptorLimits::default())
                .unwrap(),
            Some(graph.type_fingerprint(TypeInfoRef(1)).unwrap())
        );
        assert!(matches!(
            record("left").substitute(
                &BTreeMap::new(),
                TypeTemplateLimits {
                    max_text_bytes: 3,
                    ..limits()
                }
            ),
            Err(TypeTemplateError::TextLimit)
        ));
    }
}
