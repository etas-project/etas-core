//! Immutable checked operand layouts; these are not runtime values or permits.

use std::collections::BTreeMap;

use crate::identity::canonical::CanonicalEncoder;

use super::{
    identity::{BindingLayoutId, EvidenceRegisterId, FieldProjectionId, ValueBindingId},
    predicate::{BorrowedValue, ValueType},
};

/// Positions are relative to the checked semantic site, not process-local HIR IDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum BindingRoot {
    Parameter { position: u32 },
    Result,
    EvidenceRegister(EvidenceRegisterId),
}

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct BindingDescriptor {
    pub root: BindingRoot,
    pub fields: Vec<FieldProjectionId>,
    pub value_type: ValueType,
}

impl BindingDescriptor {
    pub fn id(&self, max_projections: usize) -> Result<ValueBindingId, BindingLayoutError> {
        if self.fields.len() > max_projections {
            return Err(BindingLayoutError::ProjectionLimit);
        }
        let mut encoder = CanonicalEncoder::new("etas.code.residual.value-binding.v1");
        match self.root {
            BindingRoot::Parameter { position } => {
                encoder.tag(1);
                encoder.number(u64::from(position));
            }
            BindingRoot::Result => encoder.tag(2),
            BindingRoot::EvidenceRegister(id) => {
                encoder.tag(3);
                encoder.fingerprint(id.fingerprint());
            }
        }
        encoder.number(self.fields.len() as u64);
        for field in &self.fields {
            encoder.fingerprint(field.fingerprint());
        }
        self.value_type.encode(&mut encoder);
        Ok(ValueBindingId::from_canonical_fingerprint(encoder.finish()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BindingLayoutLimits {
    pub max_bindings: usize,
    pub max_projections_per_binding: usize,
    pub max_total_projections: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingLayoutError {
    BindingLimit,
    ProjectionLimit,
    BindingIdentityMismatch(ValueBindingId),
    IdentityMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct BindingLayout {
    id: BindingLayoutId,
    bindings: BTreeMap<ValueBindingId, BindingDescriptor>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BorrowedBindingLimits {
    pub max_bindings: usize,
    pub max_value_bytes: usize,
    pub max_total_value_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BorrowedBindingError {
    BindingLimit,
    TypeMismatch(ValueBindingId),
    ValueSizeLimit(ValueBindingId),
    TotalValueSizeLimit,
}

/// A bounded view of values named by a validated layout. Missing values stay
/// missing; this view is neither evidence of approval nor a dispatch permit.
pub struct ValidatedBorrowedBindings<'a, 'value> {
    layout: &'a BindingLayout,
    values: &'a BTreeMap<ValueBindingId, BorrowedValue<'value>>,
    missing: usize,
}

impl<'value> ValidatedBorrowedBindings<'_, 'value> {
    pub fn layout_id(&self) -> BindingLayoutId {
        self.layout.id()
    }

    pub fn get(&self, id: ValueBindingId) -> Option<BorrowedValue<'value>> {
        self.layout.descriptor(id)?;
        self.values.get(&id).copied()
    }

    pub fn missing_count(&self) -> usize {
        self.missing
    }

    pub(crate) fn supplied_count(&self) -> usize {
        self.values.len()
    }

    pub(crate) fn checked_values(&self) -> impl Iterator<Item = BorrowedValue<'value>> + '_ {
        self.layout.iter().filter_map(|(id, _)| self.get(id))
    }
}

impl BindingLayout {
    pub fn new(
        bindings: BTreeMap<ValueBindingId, BindingDescriptor>,
        limits: BindingLayoutLimits,
    ) -> Result<Self, BindingLayoutError> {
        let id = checked_layout_id(&bindings, limits)?;
        Ok(Self { id, bindings })
    }

    /// Rechecks identity and stricter target limits without cloning the layout.
    pub fn validate_with_limits(
        &self,
        limits: BindingLayoutLimits,
    ) -> Result<(), BindingLayoutError> {
        if checked_layout_id(&self.bindings, limits)? != self.id {
            return Err(BindingLayoutError::IdentityMismatch);
        }
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = (ValueBindingId, &BindingDescriptor)> {
        self.bindings
            .iter()
            .map(|(id, descriptor)| (*id, descriptor))
    }

    /// Artifact reconstruction must reject a layout with a forged identity.
    pub fn from_parts(
        id: BindingLayoutId,
        bindings: BTreeMap<ValueBindingId, BindingDescriptor>,
        limits: BindingLayoutLimits,
    ) -> Result<Self, BindingLayoutError> {
        let layout = Self::new(bindings, limits)?;
        if layout.id != id {
            return Err(BindingLayoutError::IdentityMismatch);
        }
        Ok(layout)
    }

    pub fn id(&self) -> BindingLayoutId {
        self.id
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    pub fn descriptor(&self, id: ValueBindingId) -> Option<&BindingDescriptor> {
        self.bindings.get(&id)
    }

    pub fn value_type(&self, id: ValueBindingId) -> Option<ValueType> {
        self.descriptor(id).map(|descriptor| descriptor.value_type)
    }

    pub fn validate_borrowed<'a, 'value>(
        &'a self,
        values: &'a BTreeMap<ValueBindingId, BorrowedValue<'value>>,
        limits: BorrowedBindingLimits,
    ) -> Result<ValidatedBorrowedBindings<'a, 'value>, BorrowedBindingError> {
        if self.len() > limits.max_bindings || values.len() > limits.max_bindings {
            return Err(BorrowedBindingError::BindingLimit);
        }
        let mut missing = 0usize;
        let mut total = 0usize;
        for (id, descriptor) in self.iter() {
            let Some(value) = values.get(&id).copied() else {
                missing += 1;
                continue;
            };
            if value.value_type() != descriptor.value_type {
                return Err(BorrowedBindingError::TypeMismatch(id));
            }
            let bytes = value.byte_len();
            if bytes > limits.max_value_bytes {
                return Err(BorrowedBindingError::ValueSizeLimit(id));
            }
            total = total
                .checked_add(bytes)
                .ok_or(BorrowedBindingError::TotalValueSizeLimit)?;
            if total > limits.max_total_value_bytes {
                return Err(BorrowedBindingError::TotalValueSizeLimit);
            }
        }
        Ok(ValidatedBorrowedBindings {
            layout: self,
            values,
            missing,
        })
    }
}

fn checked_layout_id(
    bindings: &BTreeMap<ValueBindingId, BindingDescriptor>,
    limits: BindingLayoutLimits,
) -> Result<BindingLayoutId, BindingLayoutError> {
    if bindings.len() > limits.max_bindings {
        return Err(BindingLayoutError::BindingLimit);
    }
    let mut total = 0usize;
    for (id, descriptor) in bindings {
        total = total
            .checked_add(descriptor.fields.len())
            .ok_or(BindingLayoutError::ProjectionLimit)?;
        if total > limits.max_total_projections {
            return Err(BindingLayoutError::ProjectionLimit);
        }
        if descriptor.id(limits.max_projections_per_binding)? != *id {
            return Err(BindingLayoutError::BindingIdentityMismatch(*id));
        }
    }
    let mut encoder = CanonicalEncoder::new("etas.code.residual.binding-layout.v1");
    encoder.number(bindings.len() as u64);
    for id in bindings.keys() {
        encoder.fingerprint(id.fingerprint());
    }
    Ok(BindingLayoutId::from_canonical_fingerprint(
        encoder.finish(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodeFingerprint;

    fn limits() -> BindingLayoutLimits {
        BindingLayoutLimits {
            max_bindings: 2,
            max_projections_per_binding: 2,
            max_total_projections: 2,
        }
    }

    fn descriptor(position: u32, value_type: ValueType) -> BindingDescriptor {
        BindingDescriptor {
            root: BindingRoot::Parameter { position },
            fields: vec![],
            value_type,
        }
    }

    #[test]
    fn layout_identity_is_canonical_and_rejects_changed_bindings() {
        let a = descriptor(1, ValueType::Text);
        let b = descriptor(2, ValueType::Boolean);
        let aid = a.id(2).unwrap();
        let bid = b.id(2).unwrap();
        let first = BTreeMap::from([(aid, a.clone()), (bid, b.clone())]);
        let second = BTreeMap::from([(bid, b), (aid, a.clone())]);
        let layout = BindingLayout::new(first, limits()).unwrap();
        assert_eq!(
            layout,
            BindingLayout::new(second.clone(), limits()).unwrap()
        );
        let mut small = limits();
        small.max_bindings = 1;
        assert_eq!(
            BindingLayout::new(second.clone(), small),
            Err(BindingLayoutError::BindingLimit)
        );
        let mut forged = second;
        forged.insert(aid, descriptor(1, ValueType::Bytes));
        assert_eq!(
            BindingLayout::from_parts(layout.id(), forged, limits()),
            Err(BindingLayoutError::BindingIdentityMismatch(aid))
        );
        assert_ne!(
            a.id(2).unwrap(),
            descriptor(2, ValueType::Text).id(2).unwrap()
        );
    }

    #[test]
    fn roots_and_projections_are_distinct_and_bounded() {
        let field =
            FieldProjectionId::from_canonical_fingerprint(CodeFingerprint::from_bytes([1; 32]));
        let register =
            EvidenceRegisterId::from_canonical_fingerprint(CodeFingerprint::from_bytes([2; 32]));
        let mut value = descriptor(0, ValueType::Text);
        let parameter_id = value.id(2).unwrap();
        value.root = BindingRoot::Result;
        assert_ne!(parameter_id, value.id(2).unwrap());
        value.root = BindingRoot::EvidenceRegister(register);
        assert_ne!(parameter_id, value.id(2).unwrap());
        value.fields.push(field);
        assert_eq!(value.id(0), Err(BindingLayoutError::ProjectionLimit));
        let id = value.id(2).unwrap();
        let mut bound = limits();
        bound.max_total_projections = 0;
        assert_eq!(
            BindingLayout::new(BTreeMap::from([(id, value)]), bound),
            Err(BindingLayoutError::ProjectionLimit)
        );
    }

    #[test]
    fn borrowed_view_checks_layout_and_preserves_missing_evidence() {
        let text = descriptor(0, ValueType::Text);
        let bytes = descriptor(1, ValueType::Bytes);
        let text_id = text.id(2).unwrap();
        let bytes_id = bytes.id(2).unwrap();
        let extra_id = descriptor(2, ValueType::Text).id(2).unwrap();
        let layout = BindingLayout::new(
            BTreeMap::from([(text_id, text), (bytes_id, bytes)]),
            limits(),
        )
        .unwrap();
        let bounds = BorrowedBindingLimits {
            max_bindings: 3,
            max_value_bytes: 4,
            max_total_value_bytes: 5,
        };
        let mut values = BTreeMap::from([
            (text_id, BorrowedValue::Text("abc")),
            (extra_id, BorrowedValue::Text("extra")),
        ]);
        let view = layout.validate_borrowed(&values, bounds).unwrap();
        assert_eq!(view.missing_count(), 1);
        assert!(matches!(
            view.get(text_id),
            Some(BorrowedValue::Text("abc"))
        ));
        assert!(view.get(bytes_id).is_none());
        assert!(view.get(extra_id).is_none());

        values.insert(bytes_id, BorrowedValue::Text("x"));
        assert!(
            matches!(layout.validate_borrowed(&values, bounds), Err(BorrowedBindingError::TypeMismatch(id)) if id == bytes_id)
        );
        values.insert(bytes_id, BorrowedValue::Bytes(b"xyz"));
        assert!(matches!(
            layout.validate_borrowed(&values, bounds),
            Err(BorrowedBindingError::TotalValueSizeLimit)
        ));
        values.insert(bytes_id, BorrowedValue::Bytes(b"12"));
        assert_eq!(
            layout
                .validate_borrowed(&values, bounds)
                .unwrap()
                .missing_count(),
            0
        );
        values.insert(text_id, BorrowedValue::Text("abcde"));
        assert!(
            matches!(layout.validate_borrowed(&values, bounds), Err(BorrowedBindingError::ValueSizeLimit(id)) if id == text_id)
        );
        assert!(matches!(
            layout.validate_borrowed(
                &values,
                BorrowedBindingLimits {
                    max_bindings: 2,
                    ..bounds
                }
            ),
            Err(BorrowedBindingError::BindingLimit)
        ));
    }
}
