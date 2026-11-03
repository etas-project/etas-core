//! Canonical static action patterns, separate from runtime payload predicates.

use crate::{
    CallableId, DefinitionKey, DefinitionKind,
    contract::{CanonicalEffect, CanonicalSelector},
    descriptor::EffectTargetInfo,
    identity::{CallableIdentityRef, canonical::CanonicalEncoder},
};

use super::identity::ActionPatternId;

mod template;
pub use template::{
    ActionPatternTemplate, ActionPatternTemplateError, ActionSelectorTemplate,
    PatternTemplateLimits,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionPatternLimits {
    pub max_selectors: usize,
    pub max_identity_segments: usize,
    pub max_identity_bytes: usize,
    pub max_literal_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionPatternError {
    SelectorLimit,
    IdentityLimit,
    LiteralLimit,
    InvalidEffectOwner,
    InvalidActionName,
    NonConcreteAction,
    SelectorArityMismatch,
    IdentityMismatch,
    InvalidFamily,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionPatternMatch {
    Matches,
    Disjoint,
    NeedsConcreteSelector,
}

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct ActionPattern {
    id: ActionPatternId,
    effect: CanonicalEffect,
    family_members: Vec<DefinitionKey>,
}

struct Budget {
    limits: ActionPatternLimits,
    segments: usize,
    identity_bytes: usize,
    literal_bytes: usize,
}

impl Budget {
    fn segment(&mut self, value: &str) -> Result<(), ActionPatternError> {
        self.segments = self
            .segments
            .checked_add(1)
            .ok_or(ActionPatternError::IdentityLimit)?;
        self.identity_bytes = self
            .identity_bytes
            .checked_add(value.len())
            .ok_or(ActionPatternError::IdentityLimit)?;
        if self.segments > self.limits.max_identity_segments
            || self.identity_bytes > self.limits.max_identity_bytes
        {
            return Err(ActionPatternError::IdentityLimit);
        }
        Ok(())
    }

    fn path(&mut self, path: &[String]) -> Result<(), ActionPatternError> {
        if path.len()
            > self
                .limits
                .max_identity_segments
                .saturating_sub(self.segments)
        {
            return Err(ActionPatternError::IdentityLimit);
        }
        for segment in path {
            self.segment(segment)?;
        }
        Ok(())
    }

    fn definition(&mut self, key: &DefinitionKey) -> Result<(), ActionPatternError> {
        self.path(key.module())?;
        self.path(key.declaration())
    }

    fn callable(&mut self, callable: &CallableId) -> Result<(), ActionPatternError> {
        match callable.identity_ref() {
            CallableIdentityRef::Declaration(key) => self.definition(key),
            CallableIdentityRef::Method(method) => {
                let implementation = method.implementation();
                self.path(implementation.module())?;
                self.definition(implementation.subject())?;
                if let Some(spec) = implementation.selected_spec() {
                    self.definition(spec)?;
                }
                self.segment(method.name())
            }
        }
    }

    fn literal(&mut self, value: &str) -> Result<(), ActionPatternError> {
        self.literal_bytes = self
            .literal_bytes
            .checked_add(value.len())
            .ok_or(ActionPatternError::LiteralLimit)?;
        if self.literal_bytes > self.limits.max_literal_bytes {
            return Err(ActionPatternError::LiteralLimit);
        }
        Ok(())
    }
}

impl ActionPattern {
    /// Inputs must be canonical checked facts. This validates structure and
    /// resource use; it does not establish authority or provider support.
    pub fn new(
        effect: CanonicalEffect,
        limits: ActionPatternLimits,
    ) -> Result<Self, ActionPatternError> {
        validate_pattern(&effect, &[], limits)?;
        Ok(Self {
            id: pattern_id(&effect, &[]),
            effect,
            family_members: Vec::new(),
        })
    }

    /// `family_members` must come from the checked effect extension graph.
    /// Construction validates identity and bounds, not the graph provenance.
    pub fn from_effect_family(
        effect: CanonicalEffect,
        mut family_members: Vec<DefinitionKey>,
        limits: ActionPatternLimits,
    ) -> Result<Self, ActionPatternError> {
        if family_members.len() > limits.max_identity_segments {
            return Err(ActionPatternError::IdentityLimit);
        }
        let EffectTargetInfo::Effect(owner) = &effect.target else {
            return Err(ActionPatternError::InvalidFamily);
        };
        if !effect.arguments.is_empty() {
            return Err(ActionPatternError::InvalidFamily);
        }
        family_members.sort_unstable();
        family_members.dedup();
        family_members.retain(|member| member != owner);
        validate_pattern(&effect, &family_members, limits)?;
        Ok(Self {
            id: pattern_id(&effect, &family_members),
            effect,
            family_members,
        })
    }

    pub fn validate_with_limits(
        &self,
        limits: ActionPatternLimits,
    ) -> Result<(), ActionPatternError> {
        validate_pattern(&self.effect, &self.family_members, limits)?;
        if pattern_id(&self.effect, &self.family_members) != self.id {
            return Err(ActionPatternError::IdentityMismatch);
        }
        Ok(())
    }

    pub fn from_parts(
        id: ActionPatternId,
        effect: CanonicalEffect,
        limits: ActionPatternLimits,
    ) -> Result<Self, ActionPatternError> {
        let pattern = Self::new(effect, limits)?;
        if pattern.id != id {
            return Err(ActionPatternError::IdentityMismatch);
        }
        Ok(pattern)
    }

    pub fn from_effect_family_parts(
        id: ActionPatternId,
        effect: CanonicalEffect,
        family_members: Vec<DefinitionKey>,
        limits: ActionPatternLimits,
    ) -> Result<Self, ActionPatternError> {
        let pattern = Self::from_effect_family(effect, family_members, limits)?;
        if pattern.id != id {
            return Err(ActionPatternError::IdentityMismatch);
        }
        Ok(pattern)
    }

    pub fn id(&self) -> ActionPatternId {
        self.id
    }
    pub fn effect(&self) -> &CanonicalEffect {
        &self.effect
    }

    pub fn family_members(&self) -> &[DefinitionKey] {
        &self.family_members
    }

    /// Static coverage only; this does not evaluate payloads or grant authority.
    pub fn covers(&self, candidate: &CanonicalEffect) -> bool {
        if let EffectTargetInfo::Effect(owner) = &self.effect.target
            && self.effect.arguments.is_empty()
        {
            if matches!(candidate.target, EffectTargetInfo::Effect(_))
                && !candidate.arguments.is_empty()
            {
                return false;
            }
            let candidate_owner = match &candidate.target {
                EffectTargetInfo::Effect(owner)
                | EffectTargetInfo::Action { effect: owner, .. } => owner,
            };
            return candidate_owner == owner
                || self.family_members.binary_search(candidate_owner).is_ok();
        }
        self.effect.covers(candidate)
    }

    /// Classifies checked action selectors without treating an unresolved
    /// wildcard as proof that a narrower pattern is disjoint.
    pub fn match_action(
        &self,
        candidate: &CanonicalEffect,
        limits: ActionPatternLimits,
    ) -> Result<ActionPatternMatch, ActionPatternError> {
        validate_action_instance(candidate, limits)?;
        let EffectTargetInfo::Action {
            effect: actual_owner,
            name: actual_name,
        } = &candidate.target
        else {
            return Err(ActionPatternError::NonConcreteAction);
        };
        match &self.effect.target {
            EffectTargetInfo::Action { effect, name } => {
                if effect != actual_owner || name != actual_name {
                    return Ok(ActionPatternMatch::Disjoint);
                }
                if self.effect.arguments.len() != candidate.arguments.len() {
                    return Err(ActionPatternError::SelectorArityMismatch);
                }
            }
            EffectTargetInfo::Effect(effect) => {
                if effect != actual_owner
                    && self.family_members.binary_search(actual_owner).is_err()
                {
                    return Ok(ActionPatternMatch::Disjoint);
                }
                if self.effect.arguments.is_empty() {
                    return Ok(ActionPatternMatch::Matches);
                }
                if self.effect.arguments.len() != candidate.arguments.len() {
                    return Ok(ActionPatternMatch::NeedsConcreteSelector);
                }
            }
        }
        let mut needs_selector = false;
        for (pattern, actual) in self.effect.arguments.iter().zip(&candidate.arguments) {
            if matches!(pattern, CanonicalSelector::Wildcard) || pattern == actual {
                continue;
            }
            // Descriptor fingerprints identify graphs, not type-relation disjointness.
            if matches!(actual, CanonicalSelector::Wildcard)
                || matches!(
                    (pattern, actual),
                    (CanonicalSelector::Type(_), CanonicalSelector::Type(_))
                )
            {
                needs_selector = true;
            } else {
                return Ok(ActionPatternMatch::Disjoint);
            }
        }
        Ok(if needs_selector {
            ActionPatternMatch::NeedsConcreteSelector
        } else {
            ActionPatternMatch::Matches
        })
    }
}

fn pattern_id(effect: &CanonicalEffect, family_members: &[DefinitionKey]) -> ActionPatternId {
    let mut encoder = CanonicalEncoder::new("etas.code.residual.action-pattern.v1");
    match &effect.target {
        EffectTargetInfo::Effect(key) => {
            encoder.tag(1);
            encoder.fingerprint(key.fingerprint());
        }
        EffectTargetInfo::Action { effect, name } => {
            encoder.tag(2);
            encoder.fingerprint(effect.fingerprint());
            encoder.bytes(name.as_bytes());
        }
    }
    encoder.number(effect.arguments.len() as u64);
    for selector in &effect.arguments {
        match selector {
            CanonicalSelector::Type(ty) => {
                encoder.tag(1);
                encoder.fingerprint(*ty);
            }
            CanonicalSelector::Resource(key) => {
                encoder.tag(2);
                encoder.fingerprint(key.fingerprint());
            }
            CanonicalSelector::Callable(callable) => {
                encoder.tag(3);
                encoder.fingerprint(callable.fingerprint());
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
    encoder.number(family_members.len() as u64);
    for member in family_members {
        encoder.fingerprint(member.fingerprint());
    }
    ActionPatternId::from_canonical_fingerprint(encoder.finish())
}

/// Validates a checked action identity without cloning it. Wildcard static
/// selectors remain valid and must be classified against the pattern.
pub fn validate_action_instance(
    effect: &CanonicalEffect,
    limits: ActionPatternLimits,
) -> Result<(), ActionPatternError> {
    if !matches!(effect.target, EffectTargetInfo::Action { .. }) {
        return Err(ActionPatternError::NonConcreteAction);
    }
    validate_effect(effect, limits)
}

fn validate_effect(
    effect: &CanonicalEffect,
    limits: ActionPatternLimits,
) -> Result<(), ActionPatternError> {
    let mut budget = Budget {
        limits,
        segments: 0,
        identity_bytes: 0,
        literal_bytes: 0,
    };
    validate_effect_with_budget(effect, &mut budget)
}

fn validate_pattern(
    effect: &CanonicalEffect,
    family_members: &[DefinitionKey],
    limits: ActionPatternLimits,
) -> Result<(), ActionPatternError> {
    let mut budget = Budget {
        limits,
        segments: 0,
        identity_bytes: 0,
        literal_bytes: 0,
    };
    validate_effect_with_budget(effect, &mut budget)?;
    if !family_members.is_empty() {
        let EffectTargetInfo::Effect(owner) = &effect.target else {
            return Err(ActionPatternError::InvalidFamily);
        };
        if !effect.arguments.is_empty() {
            return Err(ActionPatternError::InvalidFamily);
        }
        let mut previous = None;
        for member in family_members {
            if member.kind() != DefinitionKind::Effect
                || member == owner
                || previous.is_some_and(|key| key >= member)
            {
                return Err(ActionPatternError::InvalidFamily);
            }
            budget.definition(member)?;
            previous = Some(member);
        }
    }
    Ok(())
}

fn validate_effect_with_budget(
    effect: &CanonicalEffect,
    budget: &mut Budget,
) -> Result<(), ActionPatternError> {
    if effect.arguments.len() > budget.limits.max_selectors {
        return Err(ActionPatternError::SelectorLimit);
    }
    validate_target_with_budget(&effect.target, budget)?;
    for selector in &effect.arguments {
        validate_selector_with_budget(selector, budget)?;
    }
    Ok(())
}

fn validate_target_with_budget(
    target: &EffectTargetInfo,
    budget: &mut Budget,
) -> Result<(), ActionPatternError> {
    let owner = match target {
        EffectTargetInfo::Effect(owner) => owner,
        EffectTargetInfo::Action {
            effect: owner,
            name,
        } => {
            if name.is_empty() || name.contains('\0') {
                return Err(ActionPatternError::InvalidActionName);
            }
            budget.segment(name)?;
            owner
        }
    };
    if owner.kind() != DefinitionKind::Effect {
        return Err(ActionPatternError::InvalidEffectOwner);
    }
    budget.definition(owner)?;
    Ok(())
}

fn validate_selector_with_budget(
    selector: &CanonicalSelector,
    budget: &mut Budget,
) -> Result<(), ActionPatternError> {
    match selector {
        CanonicalSelector::Resource(key) => budget.definition(key)?,
        CanonicalSelector::Callable(callable) => budget.callable(callable)?,
        CanonicalSelector::String(value) | CanonicalSelector::Integer(value) => {
            budget.literal(value)?
        }
        CanonicalSelector::Type(_) | CanonicalSelector::Wildcard => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CodeFingerprint, PackageKey};

    fn limits() -> ActionPatternLimits {
        ActionPatternLimits {
            max_selectors: 4,
            max_identity_segments: 8,
            max_identity_bytes: 80,
            max_literal_bytes: 12,
        }
    }

    fn effect(package: u8, name: &str) -> CanonicalEffect {
        let owner = DefinitionKey::new(
            PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([package; 32])),
            vec!["app".into()],
            vec!["Net".into()],
            DefinitionKind::Effect,
        )
        .unwrap();
        CanonicalEffect {
            target: EffectTargetInfo::Action {
                effect: owner,
                name: name.into(),
            },
            arguments: vec![CanonicalSelector::Wildcard],
        }
    }

    #[test]
    fn identity_tracks_canonical_owner_and_selector_and_rejects_tampering() {
        let source = effect(1, "connect");
        let pattern = ActionPattern::new(source.clone(), limits()).unwrap();
        let other = ActionPattern::new(effect(2, "connect"), limits()).unwrap();
        assert_ne!(pattern.id(), other.id());
        assert_eq!(
            ActionPattern::from_parts(pattern.id(), effect(2, "connect"), limits()),
            Err(ActionPatternError::IdentityMismatch)
        );
        let mut candidate = source;
        candidate.arguments = vec![CanonicalSelector::String("example".into())];
        assert!(pattern.covers(&candidate));
        assert!(!other.covers(&candidate));
        assert_ne!(
            pattern.id(),
            ActionPattern::new(candidate, limits()).unwrap().id()
        );
    }

    #[test]
    fn checked_effect_family_keeps_descendant_identity_and_any_action_arguments() {
        let EffectTargetInfo::Action { effect: root, .. } = effect(1, "connect").target else {
            unreachable!()
        };
        let child = DefinitionKey::new(
            root.package(),
            vec!["app".into()],
            vec!["Child".into()],
            DefinitionKind::Effect,
        )
        .unwrap();
        let other = DefinitionKey::new(
            root.package(),
            vec!["app".into()],
            vec!["Other".into()],
            DefinitionKind::Effect,
        )
        .unwrap();
        let source = CanonicalEffect {
            target: EffectTargetInfo::Effect(root.clone()),
            arguments: vec![],
        };
        let family = ActionPattern::from_effect_family(
            source.clone(),
            vec![child.clone(), root.clone(), child.clone()],
            limits(),
        )
        .unwrap();
        assert_eq!(family.family_members(), std::slice::from_ref(&child));
        let action = |owner: DefinitionKey| CanonicalEffect {
            target: EffectTargetInfo::Action {
                effect: owner,
                name: "connect".into(),
            },
            arguments: vec![CanonicalSelector::String("example".into())],
        };
        assert_eq!(
            family.match_action(&action(root), limits()),
            Ok(ActionPatternMatch::Matches)
        );
        assert_eq!(
            family.match_action(&action(child.clone()), limits()),
            Ok(ActionPatternMatch::Matches)
        );
        assert!(family.covers(&action(child)));
        assert_eq!(
            family.match_action(&action(other.clone()), limits()),
            Ok(ActionPatternMatch::Disjoint)
        );
        assert_ne!(
            family.id(),
            ActionPattern::new(source.clone(), limits()).unwrap().id()
        );
        assert_eq!(
            ActionPattern::from_effect_family_parts(
                family.id(),
                source.clone(),
                vec![other],
                limits()
            ),
            Err(ActionPatternError::IdentityMismatch)
        );
        assert_eq!(
            ActionPattern::from_effect_family(effect(1, "connect"), vec![], limits()),
            Err(ActionPatternError::InvalidFamily)
        );
        let mut small = limits();
        small.max_identity_segments = 2;
        assert_eq!(
            family.validate_with_limits(small),
            Err(ActionPatternError::IdentityLimit)
        );
    }

    #[test]
    fn rejects_invalid_owners_and_excessive_input_before_hashing() {
        let mut source = effect(1, "connect");
        let mut concrete_limits = limits();
        concrete_limits.max_selectors = 0;
        assert_eq!(
            validate_action_instance(&source, concrete_limits),
            Err(ActionPatternError::SelectorLimit)
        );
        let mut bound = limits();
        bound.max_identity_bytes = 2;
        assert_eq!(
            ActionPattern::new(source.clone(), bound),
            Err(ActionPatternError::IdentityLimit)
        );
        source.arguments = vec![CanonicalSelector::String("longer-than-allowed".into())];
        assert_eq!(
            ActionPattern::new(source.clone(), limits()),
            Err(ActionPatternError::LiteralLimit)
        );
        let owner = DefinitionKey::new(
            PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([1; 32])),
            vec!["app".into()],
            vec!["X".into()],
            DefinitionKind::Type,
        )
        .unwrap();
        source.target = EffectTargetInfo::Effect(owner);
        assert_eq!(
            ActionPattern::new(source, limits()),
            Err(ActionPatternError::InvalidEffectOwner)
        );
    }

    #[test]
    fn wildcard_actions_preserve_possible_overlap_without_bypassing_patterns() {
        let family = effect(1, "connect");
        let wildcard = ActionPattern::new(family.clone(), limits()).unwrap();
        assert_eq!(
            wildcard.match_action(&family, limits()),
            Ok(ActionPatternMatch::Matches)
        );
        let mut specific_effect = family.clone();
        specific_effect.arguments = vec![CanonicalSelector::String("example".into())];
        let specific = ActionPattern::new(specific_effect.clone(), limits()).unwrap();
        assert_eq!(
            specific.match_action(&family, limits()),
            Ok(ActionPatternMatch::NeedsConcreteSelector)
        );
        assert_eq!(
            specific.match_action(&specific_effect, limits()),
            Ok(ActionPatternMatch::Matches)
        );
        let mut other = family.clone();
        if let EffectTargetInfo::Action { name, .. } = &mut other.target {
            *name = "other".into();
        }
        assert_eq!(
            specific.match_action(&other, limits()),
            Ok(ActionPatternMatch::Disjoint)
        );
        other = specific_effect.clone();
        other.arguments = vec![CanonicalSelector::String("different".into())];
        assert_eq!(
            specific.match_action(&other, limits()),
            Ok(ActionPatternMatch::Disjoint)
        );
        other.arguments.clear();
        assert_eq!(
            specific.match_action(&other, limits()),
            Err(ActionPatternError::SelectorArityMismatch)
        );

        let EffectTargetInfo::Action { effect: owner, .. } = &family.target else {
            unreachable!()
        };
        let tag = ActionPattern::new(
            CanonicalEffect {
                target: EffectTargetInfo::Effect(owner.clone()),
                arguments: vec![],
            },
            limits(),
        )
        .unwrap();
        assert_eq!(
            tag.match_action(&family, limits()),
            Ok(ActionPatternMatch::Matches)
        );
    }

    #[test]
    fn distinct_type_descriptor_fingerprints_do_not_prove_selector_disjointness() {
        let mut declared = effect(1, "read");
        declared.arguments = vec![CanonicalSelector::Type(CodeFingerprint::from_bytes(
            [1; 32],
        ))];
        let pattern = ActionPattern::new(declared.clone(), limits()).unwrap();
        assert_eq!(
            pattern.match_action(&declared, limits()),
            Ok(ActionPatternMatch::Matches)
        );
        declared.arguments = vec![CanonicalSelector::Type(CodeFingerprint::from_bytes(
            [2; 32],
        ))];
        assert_eq!(
            pattern.match_action(&declared, limits()),
            Ok(ActionPatternMatch::NeedsConcreteSelector)
        );
        if let EffectTargetInfo::Action { name, .. } = &mut declared.target {
            *name = "write".into();
        }
        assert_eq!(
            pattern.match_action(&declared, limits()),
            Ok(ActionPatternMatch::Disjoint)
        );
    }
}
