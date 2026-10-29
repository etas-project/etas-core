use super::{DescriptorError, DescriptorNode, Mapping};
use crate::descriptor::{
    CallableOutcomeInfo, EffectInfo, EffectRowInfo, EffectRowInfoRef, GenericParameterInfo,
    GenericParameterRef, SignatureInfo, StaticEffectArgInfo, TraceInfo, TraceInfoRef, TypeInfo,
    TypeInfoRef, VariantPayloadInfo,
};

pub(super) struct Remapper<'a> {
    pub mapping: &'a Mapping,
    pub from: DescriptorNode,
}

impl Remapper<'_> {
    fn missing(&self, target: DescriptorNode) -> DescriptorError {
        DescriptorError::MissingReference {
            from: self.from,
            target,
        }
    }

    fn type_ref(&self, reference: &mut TypeInfoRef) -> Result<(), DescriptorError> {
        *reference = self
            .mapping
            .types
            .get(reference)
            .copied()
            .ok_or_else(|| self.missing(DescriptorNode::Type(*reference)))?;
        Ok(())
    }

    fn row_ref(&self, reference: &mut EffectRowInfoRef) -> Result<(), DescriptorError> {
        *reference = self
            .mapping
            .rows
            .get(reference)
            .copied()
            .ok_or_else(|| self.missing(DescriptorNode::Row(*reference)))?;
        Ok(())
    }

    fn binder_ref(&self, reference: &mut GenericParameterRef) -> Result<(), DescriptorError> {
        *reference = self
            .mapping
            .binders
            .get(reference)
            .copied()
            .ok_or_else(|| self.missing(DescriptorNode::Binder(*reference)))?;
        Ok(())
    }

    fn trace_ref(&self, reference: &mut TraceInfoRef) -> Result<(), DescriptorError> {
        *reference = self
            .mapping
            .traces
            .get(reference)
            .copied()
            .ok_or_else(|| self.missing(DescriptorNode::Trace(*reference)))?;
        Ok(())
    }

    pub fn ty(&self, ty: &mut TypeInfo) -> Result<(), DescriptorError> {
        match ty {
            TypeInfo::Primitive(_) => {}
            TypeInfo::Builtin { arguments, .. } | TypeInfo::Tuple(arguments) => {
                for ty in arguments {
                    self.type_ref(ty)?;
                }
            }
            TypeInfo::Parameter(reference) => self.binder_ref(reference)?,
            TypeInfo::Record(fields) => {
                for field in fields {
                    self.type_ref(&mut field.ty)?;
                }
            }
            TypeInfo::Nominal {
                parameters,
                representation,
                ..
            } => {
                for parameter in parameters {
                    self.binder_ref(parameter)?;
                }
                if let Some(ty) = representation {
                    self.type_ref(ty)?;
                }
            }
            TypeInfo::Alias {
                parameters, target, ..
            } => {
                for parameter in parameters {
                    self.binder_ref(parameter)?;
                }
                self.type_ref(target)?;
            }
            TypeInfo::Enum {
                parameters,
                variants,
                ..
            } => {
                for parameter in parameters {
                    self.binder_ref(parameter)?;
                }
                for variant in variants {
                    match &mut variant.payload {
                        VariantPayloadInfo::None => {}
                        VariantPayloadInfo::Positional(types) => {
                            for ty in types {
                                self.type_ref(ty)?;
                            }
                        }
                        VariantPayloadInfo::Named(fields) => {
                            for field in fields {
                                self.type_ref(&mut field.ty)?;
                            }
                        }
                    }
                }
            }
            TypeInfo::Applied {
                constructor,
                arguments,
            } => {
                self.type_ref(constructor)?;
                for ty in arguments {
                    self.type_ref(ty)?;
                }
            }
            TypeInfo::Function(signature) => self.signature(signature)?,
            TypeInfo::Handler {
                handled,
                produced,
                result,
            } => {
                self.row_ref(handled)?;
                if let Some(row) = produced {
                    self.row_ref(row)?;
                }
                if let Some(ty) = result {
                    self.type_ref(ty)?;
                }
            }
        }
        Ok(())
    }

    fn signature(&self, signature: &mut SignatureInfo) -> Result<(), DescriptorError> {
        for parameter in &mut signature.generics {
            self.binder_ref(parameter)?;
        }
        for parameter in &mut signature.parameters {
            self.type_ref(&mut parameter.ty)?;
        }
        self.type_ref(&mut signature.output)?;
        if let Some(ty) = &mut signature.declared_output {
            self.type_ref(ty)?;
        }
        if let Some(row) = &mut signature.effects {
            self.row_ref(row)?;
        }
        Ok(())
    }

    pub fn binder(&self, binder: &mut GenericParameterInfo) -> Result<(), DescriptorError> {
        for bound in &mut binder.bounds {
            for argument in &mut bound.arguments {
                self.type_ref(argument)?;
            }
        }
        Ok(())
    }

    pub fn row(&self, row: &mut EffectRowInfo) -> Result<(), DescriptorError> {
        if let Some(parameter) = &mut row.tail {
            self.binder_ref(parameter)?;
        }
        for effect in &mut row.effects {
            self.effect(effect)?;
        }
        Ok(())
    }

    fn effect(&self, effect: &mut EffectInfo) -> Result<(), DescriptorError> {
        for argument in &mut effect.arguments {
            if let StaticEffectArgInfo::Type(ty) = argument {
                self.type_ref(ty)?;
            }
        }
        Ok(())
    }

    pub fn trace(&self, trace: &mut TraceInfo) -> Result<(), DescriptorError> {
        match trace {
            TraceInfo::Empty | TraceInfo::Outcome(CallableOutcomeInfo::NonReturn) => {}
            TraceInfo::Event(effect) => self.effect(effect)?,
            TraceInfo::Outcome(CallableOutcomeInfo::Returned { ty, .. })
            | TraceInfo::Outcome(CallableOutcomeInfo::Raised { error: ty }) => self.type_ref(ty)?,
            TraceInfo::Outcome(CallableOutcomeInfo::ParameterNonReturn(call)) => {
                self.type_ref(&mut call.signature)?
            }
            TraceInfo::ParameterCall(call) => self.type_ref(&mut call.signature)?,
            TraceInfo::Seq(parts) | TraceInfo::Choice(parts) => {
                for part in parts {
                    self.trace_ref(part)?;
                }
            }
            TraceInfo::Repeat(inner) => self.trace_ref(inner)?,
            TraceInfo::UnknownOrder(row) => self.row_ref(row)?,
            TraceInfo::Widened {
                actions,
                parameter_calls,
            } => {
                self.row_ref(actions)?;
                for call in parameter_calls {
                    self.type_ref(&mut call.signature)?;
                }
            }
        }
        Ok(())
    }
}
