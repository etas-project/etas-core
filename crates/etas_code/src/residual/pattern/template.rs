//! Symbolic selectors retain checked binder identity until an application is known.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    ActionPattern, ActionPatternError, ActionPatternLimits, Budget, CanonicalEffect,
    CanonicalSelector, DefinitionKey, EffectTargetInfo, validate_selector_with_budget,
    validate_target_with_budget,
};
use crate::{
    CodeFingerprint,
    descriptor::{DescriptorFingerprintBudget, DescriptorLimits},
    identity::canonical::CanonicalEncoder,
    residual::{
        identity::{
            GenericArgumentIdentity, GenericParameterId, MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS,
        },
        type_binding::TypeArgumentDescriptors,
        type_template::{
            StaticTypeNode, StaticTypeTemplate, TypeTemplateError, TypeTemplateLimits,
        },
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionSelectorTemplate {
    Fixed(CanonicalSelector),
    TypeParameter(GenericParameterId),
    TypeTemplate(StaticTypeTemplate),
}

/// Descriptive source template. It is neither an installed monitor nor proof
/// that supplied type bindings came from the checked call site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionPatternTemplate {
    fingerprint: CodeFingerprint,
    target: EffectTargetInfo,
    selectors: Vec<ActionSelectorTemplate>,
    family_members: Vec<DefinitionKey>,
}

#[derive(Clone, Copy, Debug)]
pub struct PatternTemplateLimits {
    pub pattern: ActionPatternLimits,
    pub types: TypeTemplateLimits,
    pub descriptor: DescriptorLimits,
    pub max_binding_steps: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionPatternTemplateError {
    Pattern(ActionPatternError),
    Type(TypeTemplateError),
    MissingBinding(GenericParameterId),
    BindingCycle(GenericParameterId),
    BindingKind(GenericParameterId),
    MissingTypeDescriptor(GenericParameterId),
    TypeIdentityMismatch(GenericParameterId),
    ResourceLimit,
}

impl ActionPatternTemplate {
    pub fn new(
        target: EffectTargetInfo,
        selectors: Vec<ActionSelectorTemplate>,
        mut family_members: Vec<DefinitionKey>,
        limits: PatternTemplateLimits,
    ) -> Result<Self, ActionPatternTemplateError> {
        use ActionPatternTemplateError as Error;
        if selectors.len() > limits.pattern.max_selectors {
            return Err(Error::Pattern(ActionPatternError::SelectorLimit));
        }
        if family_members.len() > limits.pattern.max_identity_segments {
            return Err(Error::Pattern(ActionPatternError::IdentityLimit));
        }
        let mut budget = Budget {
            limits: limits.pattern,
            segments: 0,
            identity_bytes: 0,
            literal_bytes: 0,
        };
        validate_target_with_budget(&target, &mut budget).map_err(Error::Pattern)?;
        for selector in &selectors {
            match selector {
                ActionSelectorTemplate::Fixed(value) => {
                    validate_selector_with_budget(value, &mut budget).map_err(Error::Pattern)?
                }
                ActionSelectorTemplate::TypeParameter(_) => {}
                ActionSelectorTemplate::TypeTemplate(template) => {
                    template
                        .substitute(&BTreeMap::new(), limits.types)
                        .map_err(Error::Type)?;
                }
            }
        }
        family_members.sort_unstable();
        family_members.dedup();
        if !family_members.is_empty() {
            let EffectTargetInfo::Effect(owner) = &target else {
                return Err(Error::Pattern(ActionPatternError::InvalidFamily));
            };
            if !selectors.is_empty() {
                return Err(Error::Pattern(ActionPatternError::InvalidFamily));
            }
            family_members.retain(|member| member != owner);
            for member in &family_members {
                if member.kind() != crate::DefinitionKind::Effect {
                    return Err(Error::Pattern(ActionPatternError::InvalidFamily));
                }
                budget.definition(member).map_err(Error::Pattern)?;
            }
        }
        let fingerprint = template_fingerprint(&target, &selectors, &family_members);
        Ok(Self {
            fingerprint,
            target,
            selectors,
            family_members,
        })
    }

    pub fn selectors(&self) -> &[ActionSelectorTemplate] {
        &self.selectors
    }

    pub fn target(&self) -> &EffectTargetInfo {
        &self.target
    }

    pub fn fingerprint(&self) -> CodeFingerprint {
        self.fingerprint
    }

    /// Resolves all type selectors against checked application facts. Missing
    /// or symbolic bindings are errors, never implicit wildcard selectors.
    pub fn instantiate(
        &self,
        bindings: &BTreeMap<GenericParameterId, GenericArgumentIdentity>,
        type_descriptors: &TypeArgumentDescriptors,
        limits: PatternTemplateLimits,
    ) -> Result<ActionPattern, ActionPatternTemplateError> {
        let mut remaining = limits.max_binding_steps;
        self.instantiate_with_budget(bindings, type_descriptors, limits, &mut remaining)
    }

    /// Uses the caller's shared budget across all patterns in a plan.
    pub fn instantiate_with_budget(
        &self,
        bindings: &BTreeMap<GenericParameterId, GenericArgumentIdentity>,
        type_descriptors: &TypeArgumentDescriptors,
        limits: PatternTemplateLimits,
        remaining: &mut usize,
    ) -> Result<ActionPattern, ActionPatternTemplateError> {
        use ActionPatternTemplateError as Error;
        if bindings.len() > MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS {
            return Err(Error::ResourceLimit);
        }
        *remaining = (*remaining).min(limits.max_binding_steps);
        let mut arguments = Vec::with_capacity(self.selectors.len());
        for selector in &self.selectors {
            *remaining = remaining.checked_sub(1).ok_or(Error::ResourceLimit)?;
            arguments.push(match selector {
                ActionSelectorTemplate::Fixed(value) => value.clone(),
                ActionSelectorTemplate::TypeParameter(parameter) => {
                    let (_, fingerprint) = resolve_type(*parameter, bindings, remaining)?;
                    CanonicalSelector::Type(fingerprint)
                }
                ActionSelectorTemplate::TypeTemplate(template) => {
                    let mut replacements = BTreeMap::new();
                    for node in template.nodes() {
                        *remaining = remaining.checked_sub(1).ok_or(Error::ResourceLimit)?;
                        let StaticTypeNode::Parameter(parameter) = node else {
                            continue;
                        };
                        if replacements.contains_key(parameter) {
                            continue;
                        }
                        let (resolved, fingerprint) =
                            resolve_type(*parameter, bindings, remaining)?;
                        let (root, actual) = type_descriptors
                            .argument(resolved)
                            .ok_or(Error::MissingTypeDescriptor(resolved))?;
                        if actual != fingerprint {
                            return Err(Error::TypeIdentityMismatch(resolved));
                        }
                        replacements.insert(*parameter, root);
                    }
                    let data = type_descriptors.graph().data();
                    let work = [
                        data.types.len(),
                        data.rows.len(),
                        data.binders.len(),
                        data.traces.len(),
                    ]
                    .into_iter()
                    .try_fold(0usize, usize::checked_add)
                    .ok_or(Error::ResourceLimit)?;
                    *remaining = remaining.checked_sub(work).ok_or(Error::ResourceLimit)?;
                    let template = template
                        .substitute(&BTreeMap::new(), limits.types)
                        .map_err(Error::Type)?;
                    let (graph, root) = template
                        .instantiate_descriptor_graph(
                            type_descriptors.graph(),
                            &replacements,
                            limits.descriptor,
                        )
                        .map_err(Error::Type)?
                        .ok_or(Error::ResourceLimit)?;
                    let mut budget = DescriptorFingerprintBudget::new(*remaining);
                    let fingerprint = graph
                        .type_fingerprint_with_budget(root, &mut budget)
                        .map_err(|error| Error::Type(TypeTemplateError::Descriptor(error)))?;
                    *remaining = budget.remaining();
                    CanonicalSelector::Type(fingerprint)
                }
            });
        }
        let effect = CanonicalEffect {
            target: self.target.clone(),
            arguments,
        };
        if self.family_members.is_empty() {
            ActionPattern::new(effect, limits.pattern)
        } else {
            ActionPattern::from_effect_family(effect, self.family_members.clone(), limits.pattern)
        }
        .map_err(Error::Pattern)
    }
}

fn template_fingerprint(
    target: &EffectTargetInfo,
    selectors: &[ActionSelectorTemplate],
    family_members: &[DefinitionKey],
) -> CodeFingerprint {
    let mut encoder = CanonicalEncoder::new("etas.code.residual.action-pattern-template.v1");
    match target {
        EffectTargetInfo::Effect(owner) => {
            encoder.tag(1);
            encoder.fingerprint(owner.fingerprint());
        }
        EffectTargetInfo::Action { effect, name } => {
            encoder.tag(2);
            encoder.fingerprint(effect.fingerprint());
            encoder.bytes(name.as_bytes());
        }
    }
    encoder.number(selectors.len() as u64);
    for selector in selectors {
        match selector {
            ActionSelectorTemplate::Fixed(value) => {
                encoder.tag(1);
                match value {
                    CanonicalSelector::Type(value) => {
                        encoder.tag(1);
                        encoder.fingerprint(*value);
                    }
                    CanonicalSelector::Resource(value) => {
                        encoder.tag(2);
                        encoder.fingerprint(value.fingerprint());
                    }
                    CanonicalSelector::ResourceProjection { root, fields } => {
                        encoder.tag(7);
                        encoder.fingerprint(root.fingerprint());
                        encoder.path(fields);
                    }
                    CanonicalSelector::Callable(value) => {
                        encoder.tag(3);
                        encoder.fingerprint(value.fingerprint());
                    }
                    CanonicalSelector::String(value) => {
                        encoder.tag(4);
                        encoder.bytes(value.as_bytes());
                    }
                    CanonicalSelector::Integer(value) => {
                        encoder.tag(5);
                        encoder.bytes(value.as_bytes());
                    }
                    CanonicalSelector::Wildcard => encoder.tag(6),
                }
            }
            ActionSelectorTemplate::TypeParameter(value) => {
                encoder.tag(2);
                encoder.fingerprint(value.fingerprint());
            }
            ActionSelectorTemplate::TypeTemplate(value) => {
                encoder.tag(3);
                encoder.fingerprint(value.fingerprint());
            }
        }
    }
    encoder.number(family_members.len() as u64);
    for member in family_members {
        encoder.fingerprint(member.fingerprint());
    }
    encoder.finish()
}

fn resolve_type(
    start: GenericParameterId,
    bindings: &BTreeMap<GenericParameterId, GenericArgumentIdentity>,
    remaining: &mut usize,
) -> Result<(GenericParameterId, CodeFingerprint), ActionPatternTemplateError> {
    use ActionPatternTemplateError as Error;
    let mut seen = BTreeSet::new();
    let mut current = start;
    loop {
        *remaining = remaining.checked_sub(1).ok_or(Error::ResourceLimit)?;
        if !seen.insert(current) {
            return Err(Error::BindingCycle(current));
        }
        match bindings.get(&current) {
            Some(GenericArgumentIdentity::Type(fingerprint)) => return Ok((current, *fingerprint)),
            Some(GenericArgumentIdentity::TypeParameter(next)) => current = *next,
            Some(
                GenericArgumentIdentity::EffectRow(_)
                | GenericArgumentIdentity::EffectRowParameter(_),
            ) => return Err(Error::BindingKind(current)),
            None => return Err(Error::MissingBinding(current)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DefinitionKind, PackageKey,
        descriptor::{
            BuiltinTypeInfo, DescriptorGraph, DescriptorGraphInput, PrimitiveInfo, TypeInfo,
            TypeInfoRef,
        },
        residual::type_template::StaticTypeNode,
    };

    fn parameter(byte: u8) -> GenericParameterId {
        GenericParameterId::from_canonical_fingerprint(CodeFingerprint::from_bytes([byte; 32]))
    }

    fn type_arguments(
        parameter: GenericParameterId,
        primitive: PrimitiveInfo,
    ) -> TypeArgumentDescriptors {
        TypeArgumentDescriptors::new(
            DescriptorGraph::new(
                DescriptorGraphInput {
                    types: vec![TypeInfo::Primitive(primitive)],
                    ..Default::default()
                },
                limits().descriptor,
            )
            .unwrap(),
            BTreeMap::from([(parameter, TypeInfoRef(0))]),
            limits().descriptor,
        )
        .unwrap()
    }

    fn limits() -> PatternTemplateLimits {
        PatternTemplateLimits {
            pattern: ActionPatternLimits {
                max_selectors: 8,
                max_identity_segments: 16,
                max_identity_bytes: 256,
                max_literal_bytes: 256,
            },
            types: TypeTemplateLimits::default(),
            descriptor: DescriptorLimits::default(),
            max_binding_steps: 10_000,
        }
    }

    fn target() -> EffectTargetInfo {
        EffectTargetInfo::Action {
            effect: DefinitionKey::new(
                PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([7; 32])),
                vec!["app".into()],
                vec!["Gate".into()],
                DefinitionKind::Effect,
            )
            .unwrap(),
            name: "send".into(),
        }
    }

    #[test]
    fn direct_type_binders_resolve_without_widening_to_wildcard() {
        let t = parameter(1);
        let u = parameter(2);
        let actual = CodeFingerprint::from_bytes([42; 32]);
        let template = ActionPatternTemplate::new(
            target(),
            vec![ActionSelectorTemplate::TypeParameter(t)],
            vec![],
            limits(),
        )
        .unwrap();
        assert_eq!(
            template.fingerprint(),
            ActionPatternTemplate::new(
                target(),
                vec![ActionSelectorTemplate::TypeParameter(t)],
                vec![],
                limits()
            )
            .unwrap()
            .fingerprint()
        );
        assert_ne!(
            template.fingerprint(),
            ActionPatternTemplate::new(
                target(),
                vec![ActionSelectorTemplate::TypeParameter(u)],
                vec![],
                limits()
            )
            .unwrap()
            .fingerprint()
        );
        let bindings = BTreeMap::from([
            (t, GenericArgumentIdentity::TypeParameter(u)),
            (u, GenericArgumentIdentity::Type(actual)),
        ]);
        let pattern = template
            .instantiate(&bindings, &TypeArgumentDescriptors::default(), limits())
            .unwrap();
        assert_eq!(
            pattern.effect().arguments,
            vec![CanonicalSelector::Type(actual)]
        );
        assert_eq!(
            template.instantiate(
                &BTreeMap::new(),
                &TypeArgumentDescriptors::default(),
                limits()
            ),
            Err(ActionPatternTemplateError::MissingBinding(t))
        );
        let wrong = BTreeMap::from([(t, GenericArgumentIdentity::EffectRow(actual))]);
        assert_eq!(
            template.instantiate(&wrong, &TypeArgumentDescriptors::default(), limits()),
            Err(ActionPatternTemplateError::BindingKind(t))
        );
        let cycle = BTreeMap::from([
            (t, GenericArgumentIdentity::TypeParameter(u)),
            (u, GenericArgumentIdentity::TypeParameter(t)),
        ]);
        assert_eq!(
            template.instantiate(&cycle, &TypeArgumentDescriptors::default(), limits()),
            Err(ActionPatternTemplateError::BindingCycle(t))
        );
        assert_eq!(
            template.instantiate(
                &bindings,
                &TypeArgumentDescriptors::default(),
                PatternTemplateLimits {
                    max_binding_steps: 1,
                    ..limits()
                }
            ),
            Err(ActionPatternTemplateError::ResourceLimit)
        );
    }

    #[test]
    fn composite_type_binding_requires_matching_closed_structure() {
        let t = parameter(3);
        let primitive = StaticTypeTemplate::new(
            vec![StaticTypeNode::Primitive(PrimitiveInfo::I32)],
            0,
            limits().types,
        )
        .unwrap();
        let actual = primitive
            .closed_descriptor_fingerprint(limits().descriptor)
            .unwrap()
            .unwrap();
        let composite = StaticTypeTemplate::new(
            vec![
                StaticTypeNode::Parameter(t),
                StaticTypeNode::Builtin {
                    constructor: BuiltinTypeInfo::Array,
                    arguments: vec![0],
                },
            ],
            1,
            limits().types,
        )
        .unwrap();
        let template = ActionPatternTemplate::new(
            target(),
            vec![ActionSelectorTemplate::TypeTemplate(composite.clone())],
            vec![],
            limits(),
        )
        .unwrap();
        let bindings = BTreeMap::from([(t, GenericArgumentIdentity::Type(actual))]);
        assert_eq!(
            template.instantiate(&bindings, &TypeArgumentDescriptors::default(), limits()),
            Err(ActionPatternTemplateError::MissingTypeDescriptor(t))
        );
        let wrong = type_arguments(t, PrimitiveInfo::Bool);
        assert_eq!(
            template.instantiate(&bindings, &wrong, limits()),
            Err(ActionPatternTemplateError::TypeIdentityMismatch(t))
        );
        let expected = composite
            .substitute(&BTreeMap::from([(t, primitive.clone())]), limits().types)
            .unwrap()
            .closed_descriptor_fingerprint(limits().descriptor)
            .unwrap()
            .unwrap();
        let restricted = PatternTemplateLimits {
            types: TypeTemplateLimits {
                max_nodes: 0,
                ..limits().types
            },
            ..limits()
        };
        assert_eq!(
            template.instantiate(
                &bindings,
                &type_arguments(t, PrimitiveInfo::I32),
                restricted
            ),
            Err(ActionPatternTemplateError::Type(
                TypeTemplateError::NodeLimit
            ))
        );
        let pattern = template
            .instantiate(&bindings, &type_arguments(t, PrimitiveInfo::I32), limits())
            .unwrap();
        assert_eq!(
            pattern.effect().arguments,
            vec![CanonicalSelector::Type(expected)]
        );
    }
}
