//! Descriptive binding of an immutable semantic plan to a finalized revision.
//! This does not validate native sites, backend support, or installation.

use std::collections::BTreeSet;

use crate::{
    CodeFingerprint, RevisionInfo, RevisionKey,
    identity::canonical::CanonicalEncoder,
    residual::{
        identity::{ObligationId, PlanHash, ResidualObligationRef},
        plan::SemanticResidualPlan,
    },
};

const SCHEMA_VERSION: u32 = 2;

/// Untrusted transport representation. Decode through `BoundResidualPlan::from_data`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundResidualPlanData {
    pub schema_version: u32,
    pub plan: PlanHash,
    pub revision: RevisionKey,
    pub contract: CodeFingerprint,
    pub environment: CodeFingerprint,
    pub compilation: CodeFingerprint,
    pub references: Vec<CodeFingerprint>,
    pub fingerprint: CodeFingerprint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundResidualPlanError {
    UnsupportedSchema(u32),
    ObligationLimit,
    MissingObligation(ObligationId),
    UnexpectedObligation(ObligationId),
    MissingComponent(ObligationId),
    InvalidSubstitution(ObligationId),
    PlanMismatch,
    RevisionMismatch,
    ContractMismatch,
    EnvironmentMismatch,
    CompilationMismatch,
    FingerprintMismatch,
}

/// Exact plan/revision coordinates. HIR owner and site relationships and the
/// validity of the caller's expected obligations require independent checks.
/// This value is never backend or native installation evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundResidualPlan {
    data: BoundResidualPlanData,
}

impl BoundResidualPlan {
    pub fn bind(
        plan: &SemanticResidualPlan,
        revision: &RevisionInfo,
        expected: &BTreeSet<ResidualObligationRef>,
        max_obligations: usize,
    ) -> Result<Self, BoundResidualPlanError> {
        if expected.len() > max_obligations || plan.obligations().len() > max_obligations {
            return Err(BoundResidualPlanError::ObligationLimit);
        }
        let mut expected_ids = BTreeSet::new();
        for reference in expected {
            let obligation = reference.obligation();
            let declaration = plan
                .obligations()
                .get(obligation)
                .ok_or(BoundResidualPlanError::MissingObligation(obligation))?;
            if !declaration.accepts_reference(reference) {
                return Err(BoundResidualPlanError::InvalidSubstitution(obligation));
            }
            if plan.component_for(obligation).is_none() {
                return Err(BoundResidualPlanError::MissingComponent(obligation));
            }
            expected_ids.insert(obligation);
        }
        for obligation in plan.obligations().iter() {
            if !expected_ids.contains(&obligation.id()) {
                return Err(BoundResidualPlanError::UnexpectedObligation(
                    obligation.id(),
                ));
            }
        }
        let inputs = revision.inputs();
        let references: Vec<_> = expected
            .iter()
            .map(ResidualObligationRef::fingerprint)
            .collect();
        let data = BoundResidualPlanData {
            schema_version: SCHEMA_VERSION,
            plan: plan.hash(),
            revision: revision.key(),
            contract: inputs.contract,
            environment: inputs.environment,
            compilation: inputs.compilation,
            fingerprint: fingerprint(
                plan.hash(),
                revision.key(),
                inputs.contract,
                inputs.environment,
                inputs.compilation,
                &references,
            ),
            references,
        };
        Ok(Self { data })
    }

    pub fn from_data(
        data: BoundResidualPlanData,
        plan: &SemanticResidualPlan,
        revision: &RevisionInfo,
        expected: &BTreeSet<ResidualObligationRef>,
        max_obligations: usize,
    ) -> Result<Self, BoundResidualPlanError> {
        if data.schema_version != SCHEMA_VERSION {
            return Err(BoundResidualPlanError::UnsupportedSchema(
                data.schema_version,
            ));
        }
        let checked = Self::bind(plan, revision, expected, max_obligations)?;
        if data.plan != checked.data.plan {
            return Err(BoundResidualPlanError::PlanMismatch);
        }
        if data.revision != checked.data.revision {
            return Err(BoundResidualPlanError::RevisionMismatch);
        }
        if data.contract != checked.data.contract {
            return Err(BoundResidualPlanError::ContractMismatch);
        }
        if data.environment != checked.data.environment {
            return Err(BoundResidualPlanError::EnvironmentMismatch);
        }
        if data.compilation != checked.data.compilation {
            return Err(BoundResidualPlanError::CompilationMismatch);
        }
        if data.references != checked.data.references
            || data.fingerprint != checked.data.fingerprint
        {
            return Err(BoundResidualPlanError::FingerprintMismatch);
        }
        Ok(checked)
    }

    pub fn data(&self) -> &BoundResidualPlanData {
        &self.data
    }
    pub fn into_data(self) -> BoundResidualPlanData {
        self.data
    }
}

fn fingerprint(
    plan: PlanHash,
    revision: RevisionKey,
    contract: CodeFingerprint,
    environment: CodeFingerprint,
    compilation: CodeFingerprint,
    references: &[CodeFingerprint],
) -> CodeFingerprint {
    let mut encoder = CanonicalEncoder::new("etas.code.residual.bound-plan.v2");
    encoder.number(u64::from(SCHEMA_VERSION));
    encoder.fingerprint(plan.fingerprint());
    encoder.fingerprint(revision.fingerprint());
    encoder.fingerprint(contract);
    encoder.fingerprint(environment);
    encoder.fingerprint(compilation);
    encoder.number(references.len() as u64);
    for reference in references {
        encoder.fingerprint(*reference);
    }
    encoder.finish()
}
