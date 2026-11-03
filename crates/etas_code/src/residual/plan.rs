//! Immutable semantic component plans. A plan is not bound to a final revision,
//! validated native sites, backend capability, installation, or authority.

use std::collections::{BTreeMap, BTreeSet};

mod bound;
mod scope;
pub use bound::{BoundResidualPlan, BoundResidualPlanData, BoundResidualPlanError};
pub use scope::{
    ResidualActionAttempt, ResidualComponentAttempt, ResidualComponentStateData,
    ResidualScopeDecision, ResidualScopeDelta, ResidualScopeError, ResidualScopeEvent,
    ResidualScopePending, ResidualScopeState, ResidualScopeStateData, SpecializedResidualScope,
};

use crate::identity::canonical::CanonicalEncoder;

use super::{
    binding::{BindingLayout, BindingLayoutError, BindingLayoutLimits},
    identity::{
        BindingLayoutId, MonitorTemplateId, ObligationId, ObligationIdentityError,
        ObligationIdentityLimits, PlanHash,
    },
    model::{ResidualKind, ResidualObligation, ResidualObligationSet},
    monitor::{CompiledActionPredicate, CompiledTraceComponent},
};

pub(crate) const PLAN_SCHEMA_VERSION: u32 = 1;
pub(crate) const PLAN_SEMANTIC_VERSION: u32 = 1;

pub enum SemanticPlanComponent {
    Action(Box<CompiledActionPredicate>),
    Trace(Box<CompiledTraceComponent>),
}

impl SemanticPlanComponent {
    pub fn id(&self) -> MonitorTemplateId {
        match self {
            Self::Action(value) => value.id(),
            Self::Trace(value) => value.id(),
        }
    }
    pub fn obligation(&self) -> &ResidualObligation {
        match self {
            Self::Action(value) => value.obligation(),
            Self::Trace(value) => value.obligation(),
        }
    }
    fn tag(&self) -> u8 {
        match self {
            Self::Action(_) => 1,
            Self::Trace(_) => 2,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SemanticPlanLimits {
    pub max_obligations: usize,
    pub max_components: usize,
    pub max_layouts: usize,
    pub max_mappings: usize,
    /// Total plan entries, generic bindings, layout bindings and projections.
    /// Already compiled component internals have their own validation limits.
    pub max_plan_nodes: usize,
    pub identity: ObligationIdentityLimits,
    pub layout: BindingLayoutLimits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticPlanMapping {
    pub obligation: ObligationId,
    pub component: MonitorTemplateId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SemanticPlanError {
    Empty,
    ObligationLimit,
    ComponentLimit,
    LayoutLimit,
    MappingLimit,
    PlanNodeLimit,
    InvalidObligation(ObligationIdentityError),
    InvalidLayout(BindingLayoutError),
    UnsupportedKind(ObligationId),
    DuplicateComponent(MonitorTemplateId),
    DuplicateLayout(BindingLayoutId),
    DuplicateObligation(ObligationId),
    UnexpectedObligation(ObligationId),
    MissingObligation(ObligationId),
    MissingComponent(MonitorTemplateId),
    MissingLayout(BindingLayoutId),
    IncompatibleComponent(ObligationId),
    UnusedComponent(MonitorTemplateId),
    UnusedLayout(BindingLayoutId),
    HashMismatch,
    LimitEncoding,
}

/// The complete checked links for the supplied component families. This object
/// does not prove that native sites cover the obligations or that it can run.
pub struct SemanticResidualPlan {
    hash: PlanHash,
    obligations: ResidualObligationSet,
    components: BTreeMap<MonitorTemplateId, SemanticPlanComponent>,
    layouts: BTreeMap<BindingLayoutId, BindingLayout>,
    mappings: BTreeMap<ObligationId, MonitorTemplateId>,
}

impl SemanticResidualPlan {
    pub fn from_parts(
        expected: PlanHash,
        obligations: ResidualObligationSet,
        components: Vec<SemanticPlanComponent>,
        layouts: Vec<BindingLayout>,
        mappings: Vec<SemanticPlanMapping>,
        limits: SemanticPlanLimits,
    ) -> Result<Self, SemanticPlanError> {
        let plan = Self::build(obligations, components, layouts, mappings, limits)?;
        if plan.hash != expected {
            return Err(SemanticPlanError::HashMismatch);
        }
        Ok(plan)
    }

    pub fn build(
        obligations: ResidualObligationSet,
        components: Vec<SemanticPlanComponent>,
        layouts: Vec<BindingLayout>,
        mappings: Vec<SemanticPlanMapping>,
        limits: SemanticPlanLimits,
    ) -> Result<Self, SemanticPlanError> {
        if obligations.is_empty() {
            return Err(SemanticPlanError::Empty);
        }
        if obligations.len() > limits.max_obligations {
            return Err(SemanticPlanError::ObligationLimit);
        }
        if components.len() > limits.max_components {
            return Err(SemanticPlanError::ComponentLimit);
        }
        if layouts.len() > limits.max_layouts {
            return Err(SemanticPlanError::LayoutLimit);
        }
        if mappings.len() > limits.max_mappings {
            return Err(SemanticPlanError::MappingLimit);
        }
        let mut remaining = limits.max_plan_nodes;
        for count in [
            obligations.len(),
            components.len(),
            layouts.len(),
            mappings.len(),
        ] {
            charge_nodes(&mut remaining, count)?;
        }
        for obligation in obligations.iter() {
            charge_nodes(&mut remaining, obligation.identity().static_bindings.len())?;
            obligation
                .id()
                .validate(obligation.identity(), limits.identity)
                .map_err(SemanticPlanError::InvalidObligation)?;
            if matches!(obligation.kind(), ResidualKind::ExistingGuard { .. }) {
                return Err(SemanticPlanError::UnsupportedKind(obligation.id()));
            }
        }
        let mut component_map = BTreeMap::new();
        for component in components {
            let id = component.id();
            if component_map.insert(id, component).is_some() {
                return Err(SemanticPlanError::DuplicateComponent(id));
            }
        }
        let mut layout_map = BTreeMap::new();
        for layout in layouts {
            charge_nodes(&mut remaining, layout.len())?;
            for (_, descriptor) in layout.iter() {
                charge_nodes(&mut remaining, descriptor.fields.len())?;
            }
            layout
                .validate_with_limits(limits.layout)
                .map_err(SemanticPlanError::InvalidLayout)?;
            let id = layout.id();
            if layout_map.insert(id, layout).is_some() {
                return Err(SemanticPlanError::DuplicateLayout(id));
            }
        }
        let mut coverage = BTreeMap::new();
        let mut used_components = BTreeSet::new();
        let mut used_layouts = BTreeSet::new();
        for mapping in mappings {
            let obligation = obligations
                .get(mapping.obligation)
                .ok_or(SemanticPlanError::UnexpectedObligation(mapping.obligation))?;
            let component = component_map
                .get(&mapping.component)
                .ok_or(SemanticPlanError::MissingComponent(mapping.component))?;
            let kind_matches = matches!(
                (obligation.kind(), component),
                (
                    ResidualKind::ActionPattern { .. },
                    SemanticPlanComponent::Action(_)
                ) | (
                    ResidualKind::TraceContract { .. },
                    SemanticPlanComponent::Trace(_)
                )
            );
            if !kind_matches || component.obligation() != obligation {
                return Err(SemanticPlanError::IncompatibleComponent(mapping.obligation));
            }
            let layout = obligation.bindings();
            if !layout_map.contains_key(&layout) {
                return Err(SemanticPlanError::MissingLayout(layout));
            }
            if coverage
                .insert(mapping.obligation, mapping.component)
                .is_some()
            {
                return Err(SemanticPlanError::DuplicateObligation(mapping.obligation));
            }
            used_components.insert(mapping.component);
            used_layouts.insert(layout);
        }
        for obligation in obligations.iter() {
            if !coverage.contains_key(&obligation.id()) {
                return Err(SemanticPlanError::MissingObligation(obligation.id()));
            }
        }
        for id in component_map.keys() {
            if !used_components.contains(id) {
                return Err(SemanticPlanError::UnusedComponent(*id));
            }
        }
        for id in layout_map.keys() {
            if !used_layouts.contains(id) {
                return Err(SemanticPlanError::UnusedLayout(*id));
            }
        }
        let mut encoder = CanonicalEncoder::new("etas.code.residual.semantic-plan.v1");
        encoder.number(u64::from(PLAN_SCHEMA_VERSION));
        encoder.number(u64::from(PLAN_SEMANTIC_VERSION));
        for limit in [
            limits.max_obligations,
            limits.max_components,
            limits.max_layouts,
            limits.max_mappings,
            limits.max_plan_nodes,
            limits.identity.max_generic_arguments,
            limits.identity.max_structural_site_depth,
            limits.layout.max_bindings,
            limits.layout.max_projections_per_binding,
            limits.layout.max_total_projections,
        ] {
            encoder.number(u64::try_from(limit).map_err(|_| SemanticPlanError::LimitEncoding)?);
        }
        encoder
            .number(u64::try_from(coverage.len()).map_err(|_| SemanticPlanError::LimitEncoding)?);
        for (obligation, component) in &coverage {
            encoder.fingerprint(obligation.fingerprint());
            encoder.tag(
                component_map
                    .get(component)
                    .ok_or(SemanticPlanError::MissingComponent(*component))?
                    .tag(),
            );
            encoder.fingerprint(component.fingerprint());
        }
        encoder
            .number(u64::try_from(layout_map.len()).map_err(|_| SemanticPlanError::LimitEncoding)?);
        for layout in layout_map.keys() {
            encoder.fingerprint(layout.fingerprint());
        }
        Ok(Self {
            hash: PlanHash::from_canonical_fingerprint(encoder.finish()),
            obligations,
            components: component_map,
            layouts: layout_map,
            mappings: coverage,
        })
    }

    pub fn hash(&self) -> PlanHash {
        self.hash
    }
    pub fn obligations(&self) -> &ResidualObligationSet {
        &self.obligations
    }
    pub fn component(&self, id: MonitorTemplateId) -> Option<&SemanticPlanComponent> {
        self.components.get(&id)
    }
    pub fn layout(&self, id: BindingLayoutId) -> Option<&BindingLayout> {
        self.layouts.get(&id)
    }
    pub fn component_for(&self, id: ObligationId) -> Option<&SemanticPlanComponent> {
        self.mappings
            .get(&id)
            .and_then(|component| self.components.get(component))
    }
}

fn charge_nodes(remaining: &mut usize, count: usize) -> Result<(), SemanticPlanError> {
    *remaining = remaining
        .checked_sub(count)
        .ok_or(SemanticPlanError::PlanNodeLimit)?;
    Ok(())
}
