use std::collections::{BTreeMap, BTreeSet};

use crate::{
    CodeFingerprint, SourceRevisionKey, StableSourceKey, descriptor::GenericParameterKind,
    identity::canonical::CanonicalEncoder,
};

macro_rules! fingerprint_key {
    ($name:ident) => {
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            serde::Serialize,
            serde::Deserialize,
            etas_core::RetainedSize,
        )]
        pub struct $name(CodeFingerprint);

        impl $name {
            pub const fn from_canonical_fingerprint(fingerprint: CodeFingerprint) -> Self {
                Self(fingerprint)
            }

            pub const fn fingerprint(self) -> CodeFingerprint {
                self.0
            }
        }
    };
}

fingerprint_key!(BehaviorContractId);
fingerprint_key!(SpecInstanceId);
fingerprint_key!(ClauseId);
fingerprint_key!(SemanticScopeId);
fingerprint_key!(ActivationSiteId);
fingerprint_key!(GenericParameterId);
fingerprint_key!(ObligationId);
fingerprint_key!(ActionPatternId);
fingerprint_key!(ValuePatternId);
fingerprint_key!(PredicateId);
fingerprint_key!(MonitorTemplateId);
fingerprint_key!(PlanHash);
fingerprint_key!(GuardProviderId);
fingerprint_key!(GuardRequirementId);
fingerprint_key!(BindingLayoutId);
fingerprint_key!(ValueBindingId);
fingerprint_key!(EvidenceTestId);
fingerprint_key!(FieldProjectionId);
fingerprint_key!(EvidenceRegisterId);
fingerprint_key!(ResidualOwnerId);

/// The semantic rule role is independent of its inferred result or monitor plan.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum ResidualRuleKind {
    ActionPattern,
    TraceContract,
    ExistingGuard,
}

impl ResidualRuleKind {
    fn tag(self) -> u8 {
        match self {
            Self::ActionPattern => 1,
            Self::TraceContract => 2,
            Self::ExistingGuard => 3,
        }
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct ResidualRuleKey {
    kind: ResidualRuleKind,
    fingerprint: CodeFingerprint,
}

impl ResidualRuleKey {
    pub fn kind(self) -> ResidualRuleKind {
        self.kind
    }

    pub fn fingerprint(self) -> CodeFingerprint {
        self.fingerprint
    }

    pub fn from_checked_source_site(
        site: &CheckedSourceStructuralSite<'_, '_>,
        kind: ResidualRuleKind,
        limits: ObligationIdentityLimits,
    ) -> Result<Self, ObligationIdentityError> {
        let mut encoder = CanonicalEncoder::new("etas.code.residual.rule.v1");
        encoder.tag(kind.tag());
        encode_checked_structural_site(&mut encoder, site, limits)?;
        Ok(Self {
            kind,
            fingerprint: encoder.finish(),
        })
    }

    pub fn from_source_site(
        site: &SourceStructuralSite<'_>,
        kind: ResidualRuleKind,
        limits: ObligationIdentityLimits,
    ) -> Result<Self, ObligationIdentityError> {
        let mut encoder = CanonicalEncoder::new("etas.code.residual.rule.v1");
        encoder.tag(kind.tag());
        encode_structural_site(&mut encoder, site, limits)?;
        Ok(Self {
            kind,
            fingerprint: encoder.finish(),
        })
    }

    /// Recompute the key from the checked source site; a serialized key alone
    /// is not evidence that it belongs to the current source snapshot.
    pub fn validate_checked_source_site(
        self,
        site: &CheckedSourceStructuralSite<'_, '_>,
        limits: ObligationIdentityLimits,
    ) -> Result<(), ObligationIdentityError> {
        if self == Self::from_checked_source_site(site, self.kind, limits)? {
            Ok(())
        } else {
            Err(ObligationIdentityError::IdentityMismatch)
        }
    }
}

/// The checked declaration role disambiguates distinct owners at one site.
#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum ResidualOwnerKind {
    Flow,
    Agent,
    Tool,
    Method,
}

impl ResidualOwnerId {
    /// The caller must bind this structural site to a checked declaration.
    pub fn from_source_declaration(
        location: &SourceRevisionLocation,
        bytes: &[u8],
        kind: ResidualOwnerKind,
    ) -> Result<Self, ObligationIdentityError> {
        location.validate_snapshot(bytes)?;
        Ok(Self::encode_source_declaration(location, kind))
    }

    pub fn from_checked_source_declaration(
        snapshot: &CheckedSourceSnapshot<'_>,
        location: &SourceRevisionLocation,
        kind: ResidualOwnerKind,
    ) -> Result<Self, ObligationIdentityError> {
        snapshot.validate_location(location)?;
        Ok(Self::encode_source_declaration(location, kind))
    }

    fn encode_source_declaration(
        location: &SourceRevisionLocation,
        kind: ResidualOwnerKind,
    ) -> Self {
        let mut encoder = CanonicalEncoder::new("etas.code.residual-owner.v1");
        encoder.fingerprint(location.source.fingerprint());
        encoder.number(location.byte_start);
        encoder.number(location.byte_end);
        encoder.tag(match kind {
            ResidualOwnerKind::Flow => 1,
            ResidualOwnerKind::Agent => 2,
            ResidualOwnerKind::Tool => 3,
            ResidualOwnerKind::Method => 4,
        });
        Self(encoder.finish())
    }
}

/// A source location in an immutable revision, using an end-exclusive byte range.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct SourceRevisionLocation {
    pub source: StableSourceKey,
    pub revision: SourceRevisionKey,
    pub byte_start: u64,
    pub byte_end: u64,
}

impl SourceRevisionLocation {
    /// The range is checked against the exact snapshot used for the revision.
    pub fn new(
        source: StableSourceKey,
        bytes: &[u8],
        byte_start: u64,
        byte_end: u64,
    ) -> Result<Self, ObligationIdentityError> {
        if byte_start > byte_end || byte_end > bytes.len() as u64 {
            return Err(ObligationIdentityError::InvalidSourceRange);
        }
        Ok(Self {
            source,
            revision: SourceRevisionKey::new(source, bytes),
            byte_start,
            byte_end,
        })
    }

    pub fn validate_snapshot(&self, bytes: &[u8]) -> Result<(), ObligationIdentityError> {
        let expected = Self::new(self.source, bytes, self.byte_start, self.byte_end)?;
        if self.revision != expected.revision {
            return Err(ObligationIdentityError::IdentityMismatch);
        }
        Ok(())
    }
}

/// An immutable source snapshot whose revision is computed once from its bytes.
#[derive(Clone, Copy)]
pub struct CheckedSourceSnapshot<'a> {
    source: StableSourceKey,
    revision: SourceRevisionKey,
    bytes: &'a [u8],
}

impl<'a> CheckedSourceSnapshot<'a> {
    pub fn new(source: StableSourceKey, bytes: &'a [u8]) -> Self {
        Self {
            source,
            revision: SourceRevisionKey::new(source, bytes),
            bytes,
        }
    }

    pub fn location(
        &self,
        byte_start: u64,
        byte_end: u64,
    ) -> Result<SourceRevisionLocation, ObligationIdentityError> {
        if byte_start > byte_end || byte_end > self.bytes.len() as u64 {
            return Err(ObligationIdentityError::InvalidSourceRange);
        }
        Ok(SourceRevisionLocation {
            source: self.source,
            revision: self.revision,
            byte_start,
            byte_end,
        })
    }

    fn validate_location(
        &self,
        location: &SourceRevisionLocation,
    ) -> Result<(), ObligationIdentityError> {
        if location.source != self.source || location.revision != self.revision {
            return Err(ObligationIdentityError::IdentityMismatch);
        }
        if location.byte_start > location.byte_end || location.byte_end > self.bytes.len() as u64 {
            return Err(ObligationIdentityError::InvalidSourceRange);
        }
        Ok(())
    }
}

/// Source constraints derive their identity from the owner, spec and clause
/// below; only an already frozen input contract may supply a contract key.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum BehaviorContractRef {
    SourceConstraint,
    FrozenInput(BehaviorContractId),
}

/// The argument kind is part of identity: type and effect-row bindings cannot alias.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub enum GenericArgumentIdentity {
    Type(CodeFingerprint),
    EffectRow(CodeFingerprint),
    TypeParameter(GenericParameterId),
    EffectRowParameter(GenericParameterId),
}

/// A source-level binding recipe, not a solved application substitution.
/// Explicit fingerprints must come from canonical source bindings rather than
/// inferred effect rows or summaries.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum StaticBindingRecipe {
    ExplicitType(CodeFingerprint),
    ExplicitEffectRow(CodeFingerprint),
    TypeParameter(GenericParameterId),
    EffectRowParameter(GenericParameterId),
    TypeTemplate(super::type_template::StaticTypeTemplate),
}

/// Absolute artifact ceiling; selected targets may impose stricter limits.
pub const MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS: usize = 1024;

/// Interprocedural summaries carry an obligation identity and checked
/// substitutions, not an executable plan or installation evidence.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct ResidualObligationRef {
    obligation: ObligationId,
    #[serde(
        serialize_with = "serialize_substitutions",
        deserialize_with = "deserialize_substitutions"
    )]
    substitutions: BTreeMap<GenericParameterId, GenericArgumentIdentity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResidualSubstitutionError {
    KindMismatch(GenericParameterId),
    Cycle(GenericParameterId),
    ResourceLimit,
}

impl ResidualObligationRef {
    pub fn new(
        obligation: ObligationId,
        substitutions: BTreeMap<GenericParameterId, GenericArgumentIdentity>,
    ) -> Result<Self, ObligationIdentityError> {
        if substitutions.len() > MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS {
            return Err(ObligationIdentityError::GenericArgumentLimit);
        }
        Ok(Self {
            obligation,
            substitutions,
        })
    }

    pub fn obligation(&self) -> ObligationId {
        self.obligation
    }

    pub fn substitutions(&self) -> &BTreeMap<GenericParameterId, GenericArgumentIdentity> {
        &self.substitutions
    }

    /// Portable identity for this obligation reference and its substitutions.
    pub fn fingerprint(&self) -> CodeFingerprint {
        let mut encoder = CanonicalEncoder::new("etas.code.residual.reference.v1");
        encoder.fingerprint(self.obligation.fingerprint());
        encoder.number(self.substitutions.len() as u64);
        for (parameter, argument) in &self.substitutions {
            encoder.fingerprint(parameter.fingerprint());
            match argument {
                GenericArgumentIdentity::Type(value) => {
                    encoder.tag(1);
                    encoder.fingerprint(*value);
                }
                GenericArgumentIdentity::EffectRow(value) => {
                    encoder.tag(2);
                    encoder.fingerprint(*value);
                }
                GenericArgumentIdentity::TypeParameter(value) => {
                    encoder.tag(3);
                    encoder.fingerprint(value.fingerprint());
                }
                GenericArgumentIdentity::EffectRowParameter(value) => {
                    encoder.tag(4);
                    encoder.fingerprint(value.fingerprint());
                }
            }
        }
        encoder.finish()
    }

    /// Composes checked substitutions without interpreting an unresolved binder
    /// as a concrete value. Missing binders remain symbolic.
    pub fn substitute(
        &self,
        bindings: &BTreeMap<GenericParameterId, GenericArgumentIdentity>,
        max_steps: usize,
    ) -> Result<Self, ResidualSubstitutionError> {
        let mut remaining = max_steps;
        self.substitute_with_budget(bindings, &mut remaining)
    }

    /// Apply one callable instantiation simultaneously. A replacement may
    /// refer to another parameter of the same owner, notably in recursive
    /// calls such as `f<U, T>`; it must not be replaced a second time here.
    pub fn instantiate_with_budget(
        &self,
        bindings: &BTreeMap<GenericParameterId, GenericArgumentIdentity>,
        remaining: &mut usize,
    ) -> Result<Self, ResidualSubstitutionError> {
        if bindings.len() > MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS {
            return Err(ResidualSubstitutionError::ResourceLimit);
        }
        let mut output = BTreeMap::new();
        for (parameter, value) in &self.substitutions {
            *remaining = remaining
                .checked_sub(1)
                .ok_or(ResidualSubstitutionError::ResourceLimit)?;
            let (binder, type_kind) = match *value {
                GenericArgumentIdentity::TypeParameter(id) => (id, true),
                GenericArgumentIdentity::EffectRowParameter(id) => (id, false),
                GenericArgumentIdentity::Type(_) | GenericArgumentIdentity::EffectRow(_) => {
                    output.insert(*parameter, *value);
                    continue;
                }
            };
            *remaining = remaining
                .checked_sub(1)
                .ok_or(ResidualSubstitutionError::ResourceLimit)?;
            let replacement = bindings.get(&binder).copied().unwrap_or(*value);
            let replacement_is_type = matches!(
                replacement,
                GenericArgumentIdentity::Type(_) | GenericArgumentIdentity::TypeParameter(_)
            );
            if type_kind != replacement_is_type {
                return Err(ResidualSubstitutionError::KindMismatch(binder));
            }
            output.insert(*parameter, replacement);
        }
        Ok(Self {
            obligation: self.obligation,
            substitutions: output,
        })
    }

    /// Compose references under one caller-owned budget shared with other
    /// obligations in the same summary.
    pub fn substitute_with_budget(
        &self,
        bindings: &BTreeMap<GenericParameterId, GenericArgumentIdentity>,
        remaining: &mut usize,
    ) -> Result<Self, ResidualSubstitutionError> {
        if bindings.len() > MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS {
            return Err(ResidualSubstitutionError::ResourceLimit);
        }
        let mut output = BTreeMap::new();
        for (parameter, value) in &self.substitutions {
            *remaining = remaining
                .checked_sub(1)
                .ok_or(ResidualSubstitutionError::ResourceLimit)?;
            let mut current = *value;
            let mut seen = BTreeSet::new();
            loop {
                let (binder, type_kind) = match current {
                    GenericArgumentIdentity::TypeParameter(id) => (id, true),
                    GenericArgumentIdentity::EffectRowParameter(id) => (id, false),
                    GenericArgumentIdentity::Type(_) | GenericArgumentIdentity::EffectRow(_) => {
                        break;
                    }
                };
                *remaining = remaining
                    .checked_sub(1)
                    .ok_or(ResidualSubstitutionError::ResourceLimit)?;
                if !seen.insert(binder) {
                    return Err(ResidualSubstitutionError::Cycle(binder));
                }
                let Some(next) = bindings.get(&binder).copied() else {
                    break;
                };
                let next_is_type = matches!(
                    next,
                    GenericArgumentIdentity::Type(_) | GenericArgumentIdentity::TypeParameter(_)
                );
                if type_kind != next_is_type {
                    return Err(ResidualSubstitutionError::KindMismatch(binder));
                }
                if next == current {
                    break;
                }
                current = next;
            }
            output.insert(*parameter, current);
        }
        Ok(Self {
            obligation: self.obligation,
            substitutions: output,
        })
    }
}

fn serialize_substitutions<S>(
    substitutions: &BTreeMap<GenericParameterId, GenericArgumentIdentity>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    if serializer.is_human_readable() {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(substitutions.len()))?;
        for entry in substitutions {
            seq.serialize_element(&entry)?;
        }
        seq.end()
    } else {
        serde::Serialize::serialize(substitutions, serializer)
    }
}

fn deserialize_substitutions<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<GenericParameterId, GenericArgumentIdentity>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = BTreeMap<GenericParameterId, GenericArgumentIdentity>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a bounded map of distinct residual generic substitutions")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut access: A,
        ) -> Result<Self::Value, A::Error> {
            use serde::de::Error;
            if access
                .size_hint()
                .is_some_and(|size| size > MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS)
            {
                return Err(A::Error::custom(
                    "residual generic substitution limit exceeded",
                ));
            }
            let mut values = BTreeMap::new();
            while let Some((parameter, value)) = access.next_entry()? {
                if values.len() >= MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS {
                    return Err(A::Error::custom(
                        "residual generic substitution limit exceeded",
                    ));
                }
                if values.insert(parameter, value).is_some() {
                    return Err(A::Error::custom("duplicate residual generic substitution"));
                }
            }
            Ok(values)
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut access: A,
        ) -> Result<Self::Value, A::Error> {
            use serde::de::Error;
            if access
                .size_hint()
                .is_some_and(|size| size > MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS)
            {
                return Err(A::Error::custom(
                    "residual generic substitution limit exceeded",
                ));
            }
            let mut values = BTreeMap::new();
            while let Some((parameter, value)) = access.next_element()? {
                if values.len() >= MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS {
                    return Err(A::Error::custom(
                        "residual generic substitution limit exceeded",
                    ));
                }
                if values.insert(parameter, value).is_some() {
                    return Err(A::Error::custom("duplicate residual generic substitution"));
                }
            }
            Ok(values)
        }
    }
    if deserializer.is_human_readable() {
        deserializer.deserialize_seq(Visitor)
    } else {
        deserializer.deserialize_map(Visitor)
    }
}

/// Inputs must come from canonical checked declarations and instantiations.
/// They are descriptive, not proof that the obligation has been verified.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct ObligationIdentityInputs {
    pub origin: SourceRevisionLocation,
    pub owner: ResidualOwnerId,
    pub contract: BehaviorContractRef,
    pub spec: SpecInstanceId,
    pub clause: ClauseId,
    pub rule: ResidualRuleKey,
    pub scope: SemanticScopeId,
    pub activation: ActivationSiteId,
    #[serde(
        serialize_with = "serialize_static_bindings",
        deserialize_with = "deserialize_static_bindings"
    )]
    pub static_bindings: BTreeMap<GenericParameterId, StaticBindingRecipe>,
}

fn serialize_static_bindings<S>(
    bindings: &BTreeMap<GenericParameterId, StaticBindingRecipe>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    if serializer.is_human_readable() {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(bindings.len()))?;
        for entry in bindings {
            seq.serialize_element(&entry)?;
        }
        seq.end()
    } else {
        serde::Serialize::serialize(bindings, serializer)
    }
}

fn deserialize_static_bindings<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<GenericParameterId, StaticBindingRecipe>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = BTreeMap<GenericParameterId, StaticBindingRecipe>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("bounded, distinct static binding recipes")
        }

        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut access: A,
        ) -> Result<Self::Value, A::Error> {
            use serde::de::Error;
            if access
                .size_hint()
                .is_some_and(|size| size > MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS)
            {
                return Err(A::Error::custom("static binding limit exceeded"));
            }
            let mut values = BTreeMap::new();
            let mut usage = (0, 0, 0);
            while let Some((parameter, value)) = access.next_entry()? {
                insert_static_binding(&mut values, &mut usage, parameter, value)
                    .map_err(A::Error::custom)?;
            }
            Ok(values)
        }

        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut access: A,
        ) -> Result<Self::Value, A::Error> {
            use serde::de::Error;
            if access
                .size_hint()
                .is_some_and(|size| size > MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS)
            {
                return Err(A::Error::custom("static binding limit exceeded"));
            }
            let mut values = BTreeMap::new();
            let mut usage = (0, 0, 0);
            while let Some((parameter, value)) = access.next_element()? {
                insert_static_binding(&mut values, &mut usage, parameter, value)
                    .map_err(A::Error::custom)?;
            }
            Ok(values)
        }
    }
    if deserializer.is_human_readable() {
        deserializer.deserialize_seq(Visitor)
    } else {
        deserializer.deserialize_map(Visitor)
    }
}

fn insert_static_binding(
    values: &mut BTreeMap<GenericParameterId, StaticBindingRecipe>,
    usage: &mut (usize, usize, usize),
    parameter: GenericParameterId,
    value: StaticBindingRecipe,
) -> Result<(), &'static str> {
    if values.len() >= MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS {
        return Err("static binding limit exceeded");
    }
    if values.contains_key(&parameter) {
        return Err("duplicate static binding");
    }
    add_static_recipe_usage(usage, &value).map_err(|_| "static recipe graph limit exceeded")?;
    values.insert(parameter, value);
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObligationIdentityError {
    InvalidSourceRange,
    IdentityMismatch,
    GenericArgumentLimit,
    StructuralSiteLimit,
    StaticRecipeLimit,
    RuleKindMismatch,
}

/// Limits are provided by the selected target or artifact boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct ObligationIdentityLimits {
    pub max_generic_arguments: usize,
    pub max_structural_site_depth: usize,
}

/// A checked structural path disambiguates sites sharing one source range.
pub struct SourceStructuralSite<'a> {
    pub location: &'a SourceRevisionLocation,
    pub source_bytes: &'a [u8],
    pub structural_path: &'a [u32],
}

pub struct CheckedSourceStructuralSite<'a, 's> {
    pub location: &'a SourceRevisionLocation,
    pub snapshot: &'a CheckedSourceSnapshot<'s>,
    pub structural_path: &'a [u32],
}

/// Retained source provenance for a materialized clause; it is not a proof of
/// plan installation and must be revalidated against the checked snapshot.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct SourceClauseSite {
    id: ClauseId,
    location: SourceRevisionLocation,
    #[serde(deserialize_with = "deserialize_bounded_source_site")]
    structural_path: Vec<u32>,
}

impl SourceClauseSite {
    pub fn from_checked_site(
        site: &CheckedSourceStructuralSite<'_, '_>,
        limits: ObligationIdentityLimits,
    ) -> Result<Self, ObligationIdentityError> {
        if site.structural_path.len() > MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS {
            return Err(ObligationIdentityError::StructuralSiteLimit);
        }
        let id = ClauseId::from_checked_source_site(site, limits)?;
        Ok(Self {
            id,
            location: site.location.clone(),
            structural_path: site.structural_path.to_vec(),
        })
    }

    pub fn id(&self) -> ClauseId {
        self.id
    }

    pub fn location(&self) -> &SourceRevisionLocation {
        &self.location
    }

    pub fn structural_path(&self) -> &[u32] {
        &self.structural_path
    }

    pub fn validate_snapshot(
        &self,
        snapshot: &CheckedSourceSnapshot<'_>,
        limits: ObligationIdentityLimits,
    ) -> Result<(), ObligationIdentityError> {
        let site = CheckedSourceStructuralSite {
            location: &self.location,
            snapshot,
            structural_path: &self.structural_path,
        };
        if ClauseId::from_checked_source_site(&site, limits)? != self.id {
            return Err(ObligationIdentityError::IdentityMismatch);
        }
        Ok(())
    }

    pub fn rule(
        &self,
        snapshot: &CheckedSourceSnapshot<'_>,
        kind: ResidualRuleKind,
        suffix: &[u32],
        limits: ObligationIdentityLimits,
    ) -> Result<ResidualRuleKey, ObligationIdentityError> {
        self.validate_snapshot(snapshot, limits)?;
        let length = self
            .structural_path
            .len()
            .checked_add(suffix.len())
            .ok_or(ObligationIdentityError::StructuralSiteLimit)?;
        if length > limits.max_structural_site_depth
            || length > MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS
        {
            return Err(ObligationIdentityError::StructuralSiteLimit);
        }
        let mut path = Vec::with_capacity(length);
        path.extend_from_slice(&self.structural_path);
        path.extend_from_slice(suffix);
        ResidualRuleKey::from_checked_source_site(
            &CheckedSourceStructuralSite {
                location: &self.location,
                snapshot,
                structural_path: &path,
            },
            kind,
            limits,
        )
    }
}

fn encode_site_fields(
    encoder: &mut CanonicalEncoder,
    location: &SourceRevisionLocation,
    path: &[u32],
) {
    encoder.fingerprint(location.source.fingerprint());
    encoder.fingerprint(location.revision.fingerprint());
    encoder.number(location.byte_start);
    encoder.number(location.byte_end);
    encoder.number(path.len() as u64);
    for index in path {
        encoder.number(u64::from(*index));
    }
}

fn encode_structural_site(
    encoder: &mut CanonicalEncoder,
    site: &SourceStructuralSite<'_>,
    limits: ObligationIdentityLimits,
) -> Result<(), ObligationIdentityError> {
    if site.structural_path.len() > limits.max_structural_site_depth {
        return Err(ObligationIdentityError::StructuralSiteLimit);
    }
    site.location.validate_snapshot(site.source_bytes)?;
    encode_site_fields(encoder, site.location, site.structural_path);
    Ok(())
}

fn encode_checked_structural_site(
    encoder: &mut CanonicalEncoder,
    site: &CheckedSourceStructuralSite<'_, '_>,
    limits: ObligationIdentityLimits,
) -> Result<(), ObligationIdentityError> {
    if site.structural_path.len() > limits.max_structural_site_depth {
        return Err(ObligationIdentityError::StructuralSiteLimit);
    }
    site.snapshot.validate_location(site.location)?;
    encode_site_fields(encoder, site.location, site.structural_path);
    Ok(())
}

impl ClauseId {
    pub fn from_checked_source_site(
        site: &CheckedSourceStructuralSite<'_, '_>,
        limits: ObligationIdentityLimits,
    ) -> Result<Self, ObligationIdentityError> {
        let mut encoder = CanonicalEncoder::new("etas.code.residual.clause-site.v1");
        encode_checked_structural_site(&mut encoder, site, limits)?;
        Ok(Self(encoder.finish()))
    }
    pub fn from_source_site(
        site: &SourceStructuralSite<'_>,
        limits: ObligationIdentityLimits,
    ) -> Result<Self, ObligationIdentityError> {
        let mut encoder = CanonicalEncoder::new("etas.code.residual.clause-site.v1");
        encode_structural_site(&mut encoder, site, limits)?;
        Ok(Self(encoder.finish()))
    }
}

impl SpecInstanceId {
    /// The caller supplies checked declaration provenance and source-level
    /// argument recipes in declaration order, never inferred row solutions.
    pub fn from_checked_source_declaration(
        site: &CheckedSourceStructuralSite<'_, '_>,
        arguments: &[StaticBindingRecipe],
        limits: ObligationIdentityLimits,
    ) -> Result<Self, ObligationIdentityError> {
        if arguments.len() > limits.max_generic_arguments {
            return Err(ObligationIdentityError::GenericArgumentLimit);
        }
        validate_static_recipe_budget(arguments.iter())?;
        let mut encoder = CanonicalEncoder::new("etas.code.residual.spec-instance.v1");
        encode_checked_structural_site(&mut encoder, site, limits)?;
        encoder.number(arguments.len() as u64);
        for argument in arguments {
            encode_static_binding(&mut encoder, argument);
        }
        Ok(Self(encoder.finish()))
    }
}

/// Source-derived spec instance inputs retained for later binding and
/// validation. Decoded facts must be checked against the original snapshot.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct SourceSpecInstance {
    id: SpecInstanceId,
    declaration: SourceRevisionLocation,
    #[serde(deserialize_with = "deserialize_bounded_source_site")]
    structural_path: Vec<u32>,
    #[serde(deserialize_with = "deserialize_bounded_static_arguments")]
    arguments: Vec<StaticBindingRecipe>,
}

fn deserialize_bounded_source_site<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<u32>, D::Error> {
    deserialize_bounded_recipe_vec(deserializer)
}

fn deserialize_bounded_static_arguments<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<StaticBindingRecipe>, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Vec<StaticBindingRecipe>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("bounded static bindings")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            use serde::de::Error;
            if seq
                .size_hint()
                .is_some_and(|len| len > MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS)
            {
                return Err(A::Error::custom("static binding limit exceeded"));
            }
            let mut values = Vec::new();
            let mut usage = (0usize, 0usize, 0usize);
            while let Some(value) = seq.next_element::<StaticBindingRecipe>()? {
                if values.len() >= MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS {
                    return Err(A::Error::custom("static binding limit exceeded"));
                }
                add_static_recipe_usage(&mut usage, &value)
                    .map_err(|_| A::Error::custom("static recipe graph limit exceeded"))?;
                values.push(value);
            }
            Ok(values)
        }
    }
    deserializer.deserialize_seq(Visitor)
}

fn add_static_recipe_usage(
    usage: &mut (usize, usize, usize),
    value: &StaticBindingRecipe,
) -> Result<(), ObligationIdentityError> {
    if let StaticBindingRecipe::TypeTemplate(template) = value {
        let (nodes, edges, text) = template.resource_usage();
        usage.0 = usage
            .0
            .checked_add(nodes)
            .ok_or(ObligationIdentityError::StaticRecipeLimit)?;
        usage.1 = usage
            .1
            .checked_add(edges)
            .ok_or(ObligationIdentityError::StaticRecipeLimit)?;
        usage.2 = usage
            .2
            .checked_add(text)
            .ok_or(ObligationIdentityError::StaticRecipeLimit)?;
        let limits = super::type_template::TypeTemplateLimits::default();
        if usage.0 > limits.max_nodes
            || usage.1 > limits.max_edges
            || usage.2 > limits.max_text_bytes
        {
            return Err(ObligationIdentityError::StaticRecipeLimit);
        }
    }
    Ok(())
}

fn validate_static_recipe_budget<'a>(
    values: impl Iterator<Item = &'a StaticBindingRecipe>,
) -> Result<(), ObligationIdentityError> {
    let mut usage = (0, 0, 0);
    for value in values {
        add_static_recipe_usage(&mut usage, value)?;
    }
    Ok(())
}

fn deserialize_bounded_recipe_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    struct Visitor<T>(std::marker::PhantomData<T>);
    impl<'de, T: serde::Deserialize<'de>> serde::de::Visitor<'de> for Visitor<T> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a bounded source identity sequence")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            use serde::de::Error;
            if seq
                .size_hint()
                .is_some_and(|len| len > MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS)
            {
                return Err(A::Error::custom("source identity sequence limit exceeded"));
            }
            let mut values = Vec::new();
            while let Some(value) = seq.next_element()? {
                if values.len() >= MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS {
                    return Err(A::Error::custom("source identity sequence limit exceeded"));
                }
                values.push(value);
            }
            Ok(values)
        }
    }
    deserializer.deserialize_seq(Visitor(std::marker::PhantomData))
}

impl SourceSpecInstance {
    pub fn from_checked_declaration(
        site: &CheckedSourceStructuralSite<'_, '_>,
        arguments: &[StaticBindingRecipe],
        limits: ObligationIdentityLimits,
    ) -> Result<Self, ObligationIdentityError> {
        if arguments.len() > MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS {
            return Err(ObligationIdentityError::GenericArgumentLimit);
        }
        if site.structural_path.len() > MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS {
            return Err(ObligationIdentityError::StructuralSiteLimit);
        }
        let id = SpecInstanceId::from_checked_source_declaration(site, arguments, limits)?;
        Ok(Self {
            id,
            declaration: site.location.clone(),
            structural_path: site.structural_path.to_vec(),
            arguments: arguments.to_vec(),
        })
    }

    pub fn validate_snapshot(
        &self,
        snapshot: &CheckedSourceSnapshot<'_>,
        limits: ObligationIdentityLimits,
    ) -> Result<(), ObligationIdentityError> {
        let site = CheckedSourceStructuralSite {
            location: &self.declaration,
            snapshot,
            structural_path: &self.structural_path,
        };
        if SpecInstanceId::from_checked_source_declaration(&site, &self.arguments, limits)?
            != self.id
        {
            return Err(ObligationIdentityError::IdentityMismatch);
        }
        Ok(())
    }

    pub fn id(&self) -> SpecInstanceId {
        self.id
    }
    pub fn declaration(&self) -> &SourceRevisionLocation {
        &self.declaration
    }
    pub fn arguments(&self) -> &[StaticBindingRecipe] {
        &self.arguments
    }
    pub fn structural_path(&self) -> &[u32] {
        &self.structural_path
    }
}

fn encode_static_binding(encoder: &mut CanonicalEncoder, argument: &StaticBindingRecipe) {
    match argument {
        StaticBindingRecipe::ExplicitType(value) => {
            encoder.tag(1);
            encoder.fingerprint(*value);
        }
        StaticBindingRecipe::ExplicitEffectRow(value) => {
            encoder.tag(2);
            encoder.fingerprint(*value);
        }
        StaticBindingRecipe::TypeParameter(value) => {
            encoder.tag(3);
            encoder.fingerprint(value.fingerprint());
        }
        StaticBindingRecipe::EffectRowParameter(value) => {
            encoder.tag(4);
            encoder.fingerprint(value.fingerprint());
        }
        StaticBindingRecipe::TypeTemplate(value) => {
            encoder.tag(5);
            encoder.fingerprint(value.fingerprint());
        }
    }
}

impl SemanticScopeId {
    pub fn from_checked_source_site(
        owner: ResidualOwnerId,
        site: &CheckedSourceStructuralSite<'_, '_>,
        limits: ObligationIdentityLimits,
    ) -> Result<Self, ObligationIdentityError> {
        let mut encoder = CanonicalEncoder::new("etas.code.residual.scope-site.v1");
        encoder.fingerprint(owner.fingerprint());
        encode_checked_structural_site(&mut encoder, site, limits)?;
        Ok(Self(encoder.finish()))
    }

    pub fn from_source_site(
        owner: ResidualOwnerId,
        site: &SourceStructuralSite<'_>,
        limits: ObligationIdentityLimits,
    ) -> Result<Self, ObligationIdentityError> {
        let mut encoder = CanonicalEncoder::new("etas.code.residual.scope-site.v1");
        encoder.fingerprint(owner.fingerprint());
        encode_structural_site(&mut encoder, site, limits)?;
        Ok(Self(encoder.finish()))
    }
}

impl ActivationSiteId {
    pub fn from_checked_source_site(
        owner: ResidualOwnerId,
        site: &CheckedSourceStructuralSite<'_, '_>,
        limits: ObligationIdentityLimits,
    ) -> Result<Self, ObligationIdentityError> {
        let mut encoder = CanonicalEncoder::new("etas.code.residual.activation-site.v1");
        encoder.fingerprint(owner.fingerprint());
        encode_checked_structural_site(&mut encoder, site, limits)?;
        Ok(Self(encoder.finish()))
    }

    pub fn from_source_site(
        owner: ResidualOwnerId,
        site: &SourceStructuralSite<'_>,
        limits: ObligationIdentityLimits,
    ) -> Result<Self, ObligationIdentityError> {
        let mut encoder = CanonicalEncoder::new("etas.code.residual.activation-site.v1");
        encoder.fingerprint(owner.fingerprint());
        encode_structural_site(&mut encoder, site, limits)?;
        Ok(Self(encoder.finish()))
    }
}

impl GenericParameterId {
    /// A source binder is identified by its checked structural owner, index and kind.
    pub fn from_source_owner(
        owner: ResidualOwnerId,
        position: u32,
        kind: GenericParameterKind,
    ) -> Self {
        let mut encoder = CanonicalEncoder::new("etas.code.residual.source-generic-parameter.v1");
        encoder.fingerprint(owner.fingerprint());
        encoder.number(u64::from(position));
        encoder.tag(match kind {
            GenericParameterKind::Type => 1,
            GenericParameterKind::EffectRow => 2,
        });
        Self(encoder.finish())
    }
}

impl ObligationId {
    /// Check a decoded identity against its inputs before consuming it.
    pub fn validate(
        self,
        inputs: &ObligationIdentityInputs,
        limits: ObligationIdentityLimits,
    ) -> Result<(), ObligationIdentityError> {
        if self == Self::from_inputs(inputs, limits)? {
            Ok(())
        } else {
            Err(ObligationIdentityError::IdentityMismatch)
        }
    }

    pub fn from_inputs(
        inputs: &ObligationIdentityInputs,
        limits: ObligationIdentityLimits,
    ) -> Result<Self, ObligationIdentityError> {
        if inputs.origin.byte_start > inputs.origin.byte_end {
            return Err(ObligationIdentityError::InvalidSourceRange);
        }
        if inputs.static_bindings.len() > limits.max_generic_arguments {
            return Err(ObligationIdentityError::GenericArgumentLimit);
        }
        validate_static_recipe_budget(inputs.static_bindings.values())?;
        let mut encoder = CanonicalEncoder::new("etas.code.residual-obligation.v2");
        encoder.fingerprint(inputs.origin.source.fingerprint());
        encoder.fingerprint(inputs.origin.revision.fingerprint());
        encoder.number(inputs.origin.byte_start);
        encoder.number(inputs.origin.byte_end);
        encoder.fingerprint(inputs.owner.fingerprint());
        match inputs.contract {
            BehaviorContractRef::SourceConstraint => encoder.tag(1),
            BehaviorContractRef::FrozenInput(key) => {
                encoder.tag(2);
                encoder.fingerprint(key.fingerprint());
            }
        }
        encoder.fingerprint(inputs.spec.fingerprint());
        encoder.fingerprint(inputs.clause.fingerprint());
        encoder.tag(inputs.rule.kind().tag());
        encoder.fingerprint(inputs.rule.fingerprint());
        encoder.fingerprint(inputs.scope.fingerprint());
        encoder.fingerprint(inputs.activation.fingerprint());
        encoder.number(inputs.static_bindings.len() as u64);
        for (parameter, argument) in &inputs.static_bindings {
            encoder.fingerprint(parameter.fingerprint());
            encode_static_binding(&mut encoder, argument);
        }
        Ok(Self(encoder.finish()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StableSourceOrigin;

    fn fp(byte: u8) -> CodeFingerprint {
        CodeFingerprint::from_bytes([byte; 32])
    }

    fn limits() -> ObligationIdentityLimits {
        ObligationIdentityLimits {
            max_generic_arguments: 8,
            max_structural_site_depth: 8,
        }
    }

    fn inputs() -> ObligationIdentityInputs {
        let source =
            StableSourceKey::new(StableSourceOrigin::Project(fp(2)), &["main.es".into()]).unwrap();
        let owner_location = SourceRevisionLocation::new(source, b"source bytes", 0, 12).unwrap();
        let owner = ResidualOwnerId::from_source_declaration(
            &owner_location,
            b"source bytes",
            ResidualOwnerKind::Flow,
        )
        .unwrap();
        ObligationIdentityInputs {
            origin: SourceRevisionLocation {
                source,
                revision: SourceRevisionKey::new(source, b"source bytes"),
                byte_start: 4,
                byte_end: 8,
            },
            owner,
            contract: BehaviorContractRef::SourceConstraint,
            spec: SpecInstanceId::from_canonical_fingerprint(fp(5)),
            clause: ClauseId::from_canonical_fingerprint(fp(6)),
            rule: ResidualRuleKey::from_source_site(
                &SourceStructuralSite {
                    location: &owner_location,
                    source_bytes: b"source bytes",
                    structural_path: &[],
                },
                ResidualRuleKind::ActionPattern,
                limits(),
            )
            .unwrap(),
            scope: SemanticScopeId::from_canonical_fingerprint(fp(7)),
            activation: ActivationSiteId::from_canonical_fingerprint(fp(8)),
            static_bindings: BTreeMap::new(),
        }
    }

    #[test]
    fn static_binding_decode_checks_duplicates_and_aggregate_template_budget() {
        use crate::descriptor::{BuiltinTypeInfo, PrimitiveInfo};
        use crate::residual::type_template::{
            StaticTypeNode, StaticTypeTemplate, TypeTemplateLimits,
        };

        let template = StaticTypeTemplate::new(
            vec![
                StaticTypeNode::Primitive(PrimitiveInfo::I32),
                StaticTypeNode::Builtin {
                    constructor: BuiltinTypeInfo::Array,
                    arguments: vec![0],
                },
            ],
            1,
            TypeTemplateLimits::default(),
        )
        .unwrap();
        let parameter = GenericParameterId::from_canonical_fingerprint(fp(10));
        let recipe = StaticBindingRecipe::TypeTemplate(template);
        let mut input = inputs();
        input.static_bindings.insert(parameter, recipe.clone());
        let mut encoded = serde_json::to_value(&input).unwrap();
        let decoded: ObligationIdentityInputs = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(decoded, input);
        assert_eq!(
            ObligationId::from_inputs(&decoded, limits()),
            ObligationId::from_inputs(&input, limits())
        );

        let bindings = encoded["static_bindings"].as_array_mut().unwrap();
        bindings.push(bindings[0].clone());
        assert!(serde_json::from_value::<ObligationIdentityInputs>(encoded).is_err());

        let mut encoded = serde_json::to_value(inputs()).unwrap();
        let bindings = encoded["static_bindings"].as_array_mut().unwrap();
        for index in 0..=TypeTemplateLimits::default().max_nodes / 2 {
            let mut bytes = [0; 32];
            bytes[..8].copy_from_slice(&(index as u64).to_le_bytes());
            let id =
                GenericParameterId::from_canonical_fingerprint(CodeFingerprint::from_bytes(bytes));
            bindings.push(serde_json::to_value((id, &recipe)).unwrap());
        }
        assert!(serde_json::from_value::<ObligationIdentityInputs>(encoded).is_err());
    }

    #[test]
    fn identity_distinguishes_scope_activation_revision_and_instantiation() {
        let base = inputs();
        let original = ObligationId::from_inputs(&base, limits()).unwrap();
        let mut changed = base.clone();
        changed.scope = SemanticScopeId::from_canonical_fingerprint(fp(9));
        assert_ne!(
            original,
            ObligationId::from_inputs(&changed, limits()).unwrap()
        );
        assert_eq!(
            original.validate(&changed, limits()),
            Err(ObligationIdentityError::IdentityMismatch)
        );
        changed = base.clone();
        changed.activation = ActivationSiteId::from_canonical_fingerprint(fp(9));
        assert_ne!(
            original,
            ObligationId::from_inputs(&changed, limits()).unwrap()
        );
        changed = base.clone();
        changed.origin.revision = SourceRevisionKey::new(changed.origin.source, b"changed");
        assert_ne!(
            original,
            ObligationId::from_inputs(&changed, limits()).unwrap()
        );
        changed = base;
        let parameter = GenericParameterId::from_canonical_fingerprint(fp(10));
        changed
            .static_bindings
            .insert(parameter, StaticBindingRecipe::ExplicitType(fp(11)));
        let with_type = ObligationId::from_inputs(&changed, limits()).unwrap();
        assert_ne!(original, with_type);
        changed
            .static_bindings
            .insert(parameter, StaticBindingRecipe::ExplicitEffectRow(fp(11)));
        assert_ne!(
            with_type,
            ObligationId::from_inputs(&changed, limits()).unwrap()
        );
    }

    #[test]
    fn obligation_identity_distinguishes_rule_site_and_kind() {
        let base = inputs();
        let location = &base.origin;
        let rule = |path: &[u32], kind| {
            ResidualRuleKey::from_source_site(
                &SourceStructuralSite {
                    location,
                    source_bytes: b"source bytes",
                    structural_path: path,
                },
                kind,
                limits(),
            )
            .unwrap()
        };
        let mut first = base.clone();
        first.rule = rule(&[0], ResidualRuleKind::ActionPattern);
        let mut second = first.clone();
        second.rule = rule(&[1], ResidualRuleKind::ActionPattern);
        assert_ne!(
            ObligationId::from_inputs(&first, limits()).unwrap(),
            ObligationId::from_inputs(&second, limits()).unwrap()
        );
        second.rule = rule(&[0], ResidualRuleKind::TraceContract);
        assert_ne!(
            ObligationId::from_inputs(&first, limits()).unwrap(),
            ObligationId::from_inputs(&second, limits()).unwrap()
        );
        let snapshot = CheckedSourceSnapshot::new(location.source, b"source bytes");
        assert_eq!(
            first.rule,
            ResidualRuleKey::from_checked_source_site(
                &CheckedSourceStructuralSite {
                    location,
                    snapshot: &snapshot,
                    structural_path: &[0],
                },
                ResidualRuleKind::ActionPattern,
                limits(),
            )
            .unwrap()
        );
        let checked_site = CheckedSourceStructuralSite {
            location,
            snapshot: &snapshot,
            structural_path: &[0],
        };
        assert_eq!(
            first
                .rule
                .validate_checked_source_site(&checked_site, limits()),
            Ok(())
        );
        let wrong_path = CheckedSourceStructuralSite {
            structural_path: &[1],
            ..checked_site
        };
        assert_eq!(
            first
                .rule
                .validate_checked_source_site(&wrong_path, limits()),
            Err(ObligationIdentityError::IdentityMismatch)
        );
        let changed = CheckedSourceSnapshot::new(location.source, b"edited bytes");
        let wrong_revision = CheckedSourceStructuralSite {
            location,
            snapshot: &changed,
            structural_path: &[0],
        };
        assert_eq!(
            first
                .rule
                .validate_checked_source_site(&wrong_revision, limits()),
            Err(ObligationIdentityError::IdentityMismatch)
        );
    }

    #[test]
    fn retained_clause_site_revalidates_snapshot_and_derives_distinct_rules() {
        let base = inputs();
        let snapshot = CheckedSourceSnapshot::new(base.origin.source, b"source bytes");
        let site = SourceClauseSite::from_checked_site(
            &CheckedSourceStructuralSite {
                location: &base.origin,
                snapshot: &snapshot,
                structural_path: &[2, 1],
            },
            limits(),
        )
        .unwrap();
        let encoded = serde_json::to_value(&site).unwrap();
        let decoded: SourceClauseSite = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(decoded, site);
        assert_eq!(decoded.validate_snapshot(&snapshot, limits()), Ok(()));
        let action = decoded
            .rule(&snapshot, ResidualRuleKind::ActionPattern, &[0], limits())
            .unwrap();
        let other = decoded
            .rule(&snapshot, ResidualRuleKind::ActionPattern, &[1], limits())
            .unwrap();
        assert_ne!(action, other);
        assert_ne!(
            action,
            decoded
                .rule(&snapshot, ResidualRuleKind::TraceContract, &[0], limits())
                .unwrap()
        );
        let mut tampered = encoded;
        tampered["structural_path"] = serde_json::json!([2, 0]);
        let tampered: SourceClauseSite = serde_json::from_value(tampered).unwrap();
        assert_eq!(
            tampered.validate_snapshot(&snapshot, limits()),
            Err(ObligationIdentityError::IdentityMismatch)
        );
        let changed = CheckedSourceSnapshot::new(base.origin.source, b"edited bytes");
        assert_eq!(
            decoded.validate_snapshot(&changed, limits()),
            Err(ObligationIdentityError::IdentityMismatch)
        );
        assert_eq!(
            decoded.rule(
                &snapshot,
                ResidualRuleKind::ActionPattern,
                &[0; 7],
                limits()
            ),
            Err(ObligationIdentityError::StructuralSiteLimit)
        );
    }

    #[test]
    fn source_location_and_contract_identity_are_preanalysis_inputs() {
        let base = inputs();
        let source = base.origin.source;
        assert_eq!(
            SourceRevisionLocation::new(source, b"abcdef", 2, 7),
            Err(ObligationIdentityError::InvalidSourceRange)
        );
        assert_eq!(
            SourceRevisionLocation::new(source, b"abcdef", 4, 3),
            Err(ObligationIdentityError::InvalidSourceRange)
        );
        let location = SourceRevisionLocation::new(source, b"abcdef", 2, 6).unwrap();
        assert_eq!(location.revision, SourceRevisionKey::new(source, b"abcdef"));
        assert_eq!(location.validate_snapshot(b"abcdef"), Ok(()));
        assert_eq!(
            location.validate_snapshot(b"ABCDEF"),
            Err(ObligationIdentityError::IdentityMismatch)
        );
        let mut frozen = base.clone();
        frozen.contract =
            BehaviorContractRef::FrozenInput(BehaviorContractId::from_canonical_fingerprint(fp(4)));
        assert_ne!(
            ObligationId::from_inputs(&base, limits()).unwrap(),
            ObligationId::from_inputs(&frozen, limits()).unwrap()
        );
    }

    #[test]
    fn residual_owner_uses_source_site_and_role_not_checked_package_identity() {
        let source = inputs().origin.source;
        let location = SourceRevisionLocation::new(source, b"flow example", 0, 12).unwrap();
        let flow = ResidualOwnerId::from_source_declaration(
            &location,
            b"flow example",
            ResidualOwnerKind::Flow,
        )
        .unwrap();
        assert_eq!(
            flow,
            ResidualOwnerId::from_source_declaration(
                &location,
                b"flow example",
                ResidualOwnerKind::Flow,
            )
            .unwrap()
        );
        assert_ne!(
            flow,
            ResidualOwnerId::from_source_declaration(
                &location,
                b"flow example",
                ResidualOwnerKind::Method,
            )
            .unwrap()
        );
        assert_eq!(
            ResidualOwnerId::from_source_declaration(
                &location,
                b"other bytes!",
                ResidualOwnerKind::Flow
            ),
            Err(ObligationIdentityError::IdentityMismatch)
        );
        let binder = GenericParameterId::from_source_owner(flow, 0, GenericParameterKind::Type);
        assert_ne!(
            binder,
            GenericParameterId::from_source_owner(flow, 1, GenericParameterKind::Type)
        );
        assert_ne!(
            binder,
            GenericParameterId::from_source_owner(flow, 0, GenericParameterKind::EffectRow)
        );
    }

    #[test]
    fn structural_site_keys_distinguish_scope_activation_and_checked_source() {
        let source = inputs().origin.source;
        let bytes = b"flow abc";
        let location = SourceRevisionLocation::new(source, bytes, 0, 8).unwrap();
        let owner =
            ResidualOwnerId::from_source_declaration(&location, bytes, ResidualOwnerKind::Flow)
                .unwrap();
        let first = SourceStructuralSite {
            location: &location,
            source_bytes: bytes,
            structural_path: &[0, 1],
        };
        let second = SourceStructuralSite {
            location: &location,
            source_bytes: bytes,
            structural_path: &[1, 0],
        };
        let clause = ClauseId::from_source_site(&first, limits()).unwrap();
        let snapshot = CheckedSourceSnapshot::new(source, bytes);
        assert_eq!(snapshot.location(0, 8).unwrap(), location);
        assert_eq!(
            ResidualOwnerId::from_checked_source_declaration(
                &snapshot,
                &location,
                ResidualOwnerKind::Flow
            ),
            Ok(owner)
        );
        let checked_site = CheckedSourceStructuralSite {
            location: &location,
            snapshot: &snapshot,
            structural_path: &[0, 1],
        };
        assert_eq!(
            ClauseId::from_checked_source_site(&checked_site, limits()),
            Ok(clause)
        );
        let edited_snapshot = CheckedSourceSnapshot::new(source, b"flow xyz");
        assert_eq!(
            ResidualOwnerId::from_checked_source_declaration(
                &edited_snapshot,
                &location,
                ResidualOwnerKind::Flow
            ),
            Err(ObligationIdentityError::IdentityMismatch)
        );
        let edited_site = CheckedSourceStructuralSite {
            location: &location,
            snapshot: &edited_snapshot,
            structural_path: &[0, 1],
        };
        assert_eq!(
            ClauseId::from_checked_source_site(&edited_site, limits()),
            Err(ObligationIdentityError::IdentityMismatch)
        );
        let mut invalid_location = location.clone();
        invalid_location.byte_end = 9;
        let invalid_site = CheckedSourceStructuralSite {
            location: &invalid_location,
            snapshot: &snapshot,
            structural_path: &[0, 1],
        };
        assert_eq!(
            ClauseId::from_checked_source_site(&invalid_site, limits()),
            Err(ObligationIdentityError::InvalidSourceRange)
        );
        assert_eq!(
            clause,
            ClauseId::from_source_site(&first, limits()).unwrap()
        );
        assert_ne!(
            clause,
            ClauseId::from_source_site(&second, limits()).unwrap()
        );
        let scope = SemanticScopeId::from_source_site(owner, &first, limits()).unwrap();
        let activation = ActivationSiteId::from_source_site(owner, &first, limits()).unwrap();
        assert_eq!(
            SemanticScopeId::from_checked_source_site(owner, &checked_site, limits()),
            Ok(scope)
        );
        assert_eq!(
            ActivationSiteId::from_checked_source_site(owner, &checked_site, limits()),
            Ok(activation)
        );
        assert_eq!(
            SemanticScopeId::from_checked_source_site(owner, &edited_site, limits()),
            Err(ObligationIdentityError::IdentityMismatch)
        );
        assert_eq!(
            ActivationSiteId::from_checked_source_site(owner, &invalid_site, limits()),
            Err(ObligationIdentityError::InvalidSourceRange)
        );
        assert_ne!(scope.fingerprint(), activation.fingerprint());
        let other_owner =
            ResidualOwnerId::from_source_declaration(&location, bytes, ResidualOwnerKind::Method)
                .unwrap();
        assert_ne!(
            scope,
            SemanticScopeId::from_source_site(other_owner, &first, limits()).unwrap()
        );
        let forged = SourceStructuralSite {
            location: &location,
            source_bytes: b"flow xyz",
            structural_path: &[0, 1],
        };
        assert_eq!(
            ClauseId::from_source_site(&forged, limits()),
            Err(ObligationIdentityError::IdentityMismatch)
        );
        let mut restricted = limits();
        restricted.max_structural_site_depth = 1;
        assert_eq!(
            ClauseId::from_source_site(&first, restricted),
            Err(ObligationIdentityError::StructuralSiteLimit)
        );
    }

    #[test]
    fn spec_instances_use_checked_declaration_and_ordered_static_recipes() {
        let source = inputs().origin.source;
        let snapshot = CheckedSourceSnapshot::new(source, b"spec Pair<T, U>");
        let location = snapshot.location(0, 15).unwrap();
        let site = CheckedSourceStructuralSite {
            location: &location,
            snapshot: &snapshot,
            structural_path: &[0],
        };
        let first = StaticBindingRecipe::ExplicitType(fp(3));
        let second = StaticBindingRecipe::TypeParameter(
            GenericParameterId::from_canonical_fingerprint(fp(4)),
        );
        let id = SpecInstanceId::from_checked_source_declaration(
            &site,
            &[first.clone(), second.clone()],
            limits(),
        )
        .unwrap();
        let fact = SourceSpecInstance::from_checked_declaration(
            &site,
            &[first.clone(), second.clone()],
            limits(),
        )
        .unwrap();
        assert_eq!(fact.id(), id);
        assert_eq!(fact.arguments(), &[first.clone(), second.clone()]);
        assert_eq!(fact.declaration(), &location);
        assert_eq!(fact.structural_path(), &[0]);
        assert_eq!(fact.validate_snapshot(&snapshot, limits()), Ok(()));
        let roundtrip: SourceSpecInstance =
            serde_json::from_str(&serde_json::to_string(&fact).unwrap()).unwrap();
        assert_eq!(roundtrip, fact);
        assert_eq!(roundtrip.validate_snapshot(&snapshot, limits()), Ok(()));
        let mut encoded = serde_json::to_value(&fact).unwrap();
        encoded["arguments"].as_array_mut().unwrap().reverse();
        let forged: SourceSpecInstance = serde_json::from_value(encoded).unwrap();
        assert_eq!(
            forged.validate_snapshot(&snapshot, limits()),
            Err(ObligationIdentityError::IdentityMismatch)
        );
        let mut oversized = serde_json::to_value(&fact).unwrap();
        oversized["structural_path"] =
            serde_json::json!(vec![0u32; MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS + 1]);
        assert!(serde_json::from_value::<SourceSpecInstance>(oversized).is_err());
        let mut oversized = serde_json::to_value(&fact).unwrap();
        oversized["arguments"] = serde_json::to_value(vec![
            first.clone();
            MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS + 1
        ])
        .unwrap();
        assert!(serde_json::from_value::<SourceSpecInstance>(oversized).is_err());
        assert_eq!(
            id,
            SpecInstanceId::from_checked_source_declaration(
                &site,
                &[first.clone(), second.clone()],
                limits()
            )
            .unwrap()
        );
        assert_ne!(
            id,
            SpecInstanceId::from_checked_source_declaration(
                &site,
                &[second.clone(), first.clone()],
                limits()
            )
            .unwrap()
        );
        assert_ne!(
            id,
            SpecInstanceId::from_checked_source_declaration(
                &site,
                &[
                    StaticBindingRecipe::ExplicitEffectRow(fp(3)),
                    second.clone()
                ],
                limits()
            )
            .unwrap()
        );
        let edited = CheckedSourceSnapshot::new(source, b"spec Pair<U, T>");
        let edited_location = edited.location(0, 15).unwrap();
        let edited_site = CheckedSourceStructuralSite {
            location: &edited_location,
            snapshot: &edited,
            structural_path: &[0],
        };
        assert_ne!(
            id,
            SpecInstanceId::from_checked_source_declaration(
                &edited_site,
                &[first.clone(), second.clone()],
                limits()
            )
            .unwrap()
        );
        let forged = CheckedSourceStructuralSite {
            location: &location,
            snapshot: &edited,
            structural_path: &[0],
        };
        assert_eq!(
            SpecInstanceId::from_checked_source_declaration(
                &forged,
                std::slice::from_ref(&first),
                limits()
            ),
            Err(ObligationIdentityError::IdentityMismatch)
        );
        let mut tight = limits();
        tight.max_generic_arguments = 1;
        assert_eq!(
            SpecInstanceId::from_checked_source_declaration(
                &site,
                &[first.clone(), second.clone()],
                tight
            ),
            Err(ObligationIdentityError::GenericArgumentLimit)
        );
    }

    #[test]
    fn generic_binding_order_is_canonical_and_invalid_ranges_are_rejected() {
        let mut first = inputs();
        let one = GenericParameterId::from_canonical_fingerprint(fp(1));
        let two = GenericParameterId::from_canonical_fingerprint(fp(2));
        first
            .static_bindings
            .insert(one, StaticBindingRecipe::ExplicitType(fp(3)));
        first
            .static_bindings
            .insert(two, StaticBindingRecipe::ExplicitEffectRow(fp(4)));
        let mut second = inputs();
        second
            .static_bindings
            .insert(two, StaticBindingRecipe::ExplicitEffectRow(fp(4)));
        second
            .static_bindings
            .insert(one, StaticBindingRecipe::ExplicitType(fp(3)));
        assert_eq!(
            ObligationId::from_inputs(&first, limits()),
            ObligationId::from_inputs(&second, limits())
        );
        second.origin.byte_end = second.origin.byte_start - 1;
        assert_eq!(
            ObligationId::from_inputs(&second, limits()),
            Err(ObligationIdentityError::InvalidSourceRange)
        );
    }

    #[test]
    fn rejects_oversized_generic_bindings_before_hashing() {
        let mut input = inputs();
        input.static_bindings.insert(
            GenericParameterId::from_canonical_fingerprint(fp(3)),
            StaticBindingRecipe::ExplicitType(fp(4)),
        );
        let mut bound = limits();
        bound.max_generic_arguments = 0;
        assert_eq!(
            ObligationId::from_inputs(&input, bound),
            Err(ObligationIdentityError::GenericArgumentLimit)
        );
    }

    #[test]
    fn generic_identity_distinguishes_structural_owner_and_binder_kind() {
        let source = inputs().origin.source;
        let bytes = b"flow one; flow two";
        let first = SourceRevisionLocation::new(source, bytes, 0, 8).unwrap();
        let second = SourceRevisionLocation::new(source, bytes, 10, 18).unwrap();
        let owner = |site: &SourceRevisionLocation| {
            ResidualOwnerId::from_source_declaration(site, bytes, ResidualOwnerKind::Flow).unwrap()
        };
        let left =
            GenericParameterId::from_source_owner(owner(&first), 0, GenericParameterKind::Type);
        assert_eq!(
            left,
            GenericParameterId::from_source_owner(owner(&first), 0, GenericParameterKind::Type)
        );
        assert_ne!(
            left,
            GenericParameterId::from_source_owner(owner(&second), 0, GenericParameterKind::Type)
        );
        assert_ne!(
            left,
            GenericParameterId::from_source_owner(owner(&first), 1, GenericParameterKind::Type)
        );
        assert_ne!(
            left,
            GenericParameterId::from_source_owner(
                owner(&first),
                0,
                GenericParameterKind::EffectRow
            )
        );
    }

    #[test]
    fn residual_call_instantiation_is_simultaneous_and_budgeted() {
        let param = |byte| GenericParameterId::from_canonical_fingerprint(fp(byte));
        let reference = ResidualObligationRef::new(
            ObligationId::from_canonical_fingerprint(fp(1)),
            BTreeMap::from([
                (param(2), GenericArgumentIdentity::TypeParameter(param(2))),
                (param(3), GenericArgumentIdentity::TypeParameter(param(3))),
                (
                    param(4),
                    GenericArgumentIdentity::EffectRowParameter(param(4)),
                ),
                (
                    param(5),
                    GenericArgumentIdentity::EffectRowParameter(param(5)),
                ),
            ]),
        )
        .unwrap();
        let bindings = BTreeMap::from([
            (param(2), GenericArgumentIdentity::TypeParameter(param(3))),
            (param(3), GenericArgumentIdentity::TypeParameter(param(2))),
            (
                param(4),
                GenericArgumentIdentity::EffectRowParameter(param(5)),
            ),
            (
                param(5),
                GenericArgumentIdentity::EffectRowParameter(param(4)),
            ),
        ]);
        let mut remaining = 16;
        let swapped = reference
            .instantiate_with_budget(&bindings, &mut remaining)
            .unwrap();
        assert_eq!(
            swapped.substitutions()[&param(2)],
            GenericArgumentIdentity::TypeParameter(param(3))
        );
        assert_eq!(
            swapped.substitutions()[&param(3)],
            GenericArgumentIdentity::TypeParameter(param(2))
        );
        assert_eq!(
            swapped.substitutions()[&param(4)],
            GenericArgumentIdentity::EffectRowParameter(param(5))
        );
        assert_eq!(
            swapped.substitutions()[&param(5)],
            GenericArgumentIdentity::EffectRowParameter(param(4))
        );
        assert_eq!(
            swapped
                .instantiate_with_budget(&bindings, &mut remaining)
                .unwrap(),
            reference
        );
        assert_eq!(remaining, 0);
        let mut remaining = 16;
        let chain = BTreeMap::from([
            (param(2), GenericArgumentIdentity::TypeParameter(param(3))),
            (param(3), GenericArgumentIdentity::Type(fp(6))),
        ]);
        let once = reference
            .instantiate_with_budget(&chain, &mut remaining)
            .unwrap();
        assert_eq!(
            once.substitutions()[&param(2)],
            GenericArgumentIdentity::TypeParameter(param(3))
        );
        assert_eq!(
            once.instantiate_with_budget(&chain, &mut remaining)
                .unwrap()
                .substitutions()[&param(2)],
            GenericArgumentIdentity::Type(fp(6))
        );
        remaining = 0;
        assert_eq!(
            reference.instantiate_with_budget(&bindings, &mut remaining),
            Err(ResidualSubstitutionError::ResourceLimit)
        );
        let wrong = BTreeMap::from([(param(2), GenericArgumentIdentity::EffectRow(fp(9)))]);
        assert_eq!(
            reference.instantiate_with_budget(&wrong, &mut 8),
            Err(ResidualSubstitutionError::KindMismatch(param(2)))
        );
    }

    #[test]
    fn residual_substitution_preserves_unresolved_and_rejects_cycles_or_kind_errors() {
        let param = |byte| GenericParameterId::from_canonical_fingerprint(fp(byte));
        let reference = ResidualObligationRef::new(
            ObligationId::from_canonical_fingerprint(fp(1)),
            BTreeMap::from([
                (param(2), GenericArgumentIdentity::TypeParameter(param(3))),
                (
                    param(4),
                    GenericArgumentIdentity::EffectRowParameter(param(5)),
                ),
            ]),
        )
        .unwrap();
        let bindings = BTreeMap::from([
            (param(3), GenericArgumentIdentity::TypeParameter(param(6))),
            (param(6), GenericArgumentIdentity::Type(fp(7))),
        ]);
        let applied = reference.substitute(&bindings, 8).unwrap();
        assert_ne!(applied.fingerprint(), reference.fingerprint());
        assert_eq!(
            applied.substitutions()[&param(2)],
            GenericArgumentIdentity::Type(fp(7))
        );
        assert_eq!(
            applied.substitutions()[&param(4)],
            GenericArgumentIdentity::EffectRowParameter(param(5))
        );
        assert_eq!(applied.obligation(), reference.obligation());
        let identity = BTreeMap::from([
            (param(3), GenericArgumentIdentity::TypeParameter(param(3))),
            (
                param(5),
                GenericArgumentIdentity::EffectRowParameter(param(5)),
            ),
        ]);
        assert_eq!(reference.substitute(&identity, 8).unwrap(), reference);
        let wrong = BTreeMap::from([(param(3), GenericArgumentIdentity::EffectRow(fp(7)))]);
        assert_eq!(
            reference.substitute(&wrong, 8),
            Err(ResidualSubstitutionError::KindMismatch(param(3)))
        );
        let cycle = BTreeMap::from([
            (param(3), GenericArgumentIdentity::TypeParameter(param(6))),
            (param(6), GenericArgumentIdentity::TypeParameter(param(3))),
        ]);
        assert_eq!(
            reference.substitute(&cycle, 8),
            Err(ResidualSubstitutionError::Cycle(param(3)))
        );
        assert_eq!(
            reference.substitute(&bindings, 1),
            Err(ResidualSubstitutionError::ResourceLimit)
        );
    }

    #[test]
    fn residual_substitution_budget_is_shared_across_references() {
        let parameter = GenericParameterId::from_canonical_fingerprint(fp(2));
        let reference = ResidualObligationRef::new(
            ObligationId::from_canonical_fingerprint(fp(1)),
            BTreeMap::from([(parameter, GenericArgumentIdentity::TypeParameter(parameter))]),
        )
        .unwrap();
        let bindings = BTreeMap::from([(parameter, GenericArgumentIdentity::Type(fp(3)))]);
        let mut remaining = 3;
        assert!(
            reference
                .substitute_with_budget(&bindings, &mut remaining)
                .is_ok()
        );
        assert_eq!(remaining, 1);
        assert_eq!(
            reference.substitute_with_budget(&bindings, &mut remaining),
            Err(ResidualSubstitutionError::ResourceLimit)
        );
    }
}
