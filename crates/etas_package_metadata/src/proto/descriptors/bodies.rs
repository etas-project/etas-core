use etas_code::{
    CodeFingerprint,
    descriptor::{BodyAccess, CallableBodyInfo, DefinitionBodies},
    syntax::{RetainedBodyForm, RetainedBodySource},
};

use super::{identities::*, invalid, required, schema::Callable};
use crate::MetadataArtifactError as Error;

#[derive(Clone, PartialEq, prost::Message)]
pub(in crate::proto) struct Bodies {
    #[prost(message, repeated, tag = "1")]
    pub entries: Vec<Body>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub(in crate::proto) struct Body {
    #[prost(message, optional, tag = "1")]
    pub callable: Option<Callable>,
    #[prost(uint32, tag = "2")]
    pub access: u32,
    #[prost(uint32, tag = "3")]
    pub syntax_schema: u32,
    #[prost(uint32, tag = "4")]
    pub form: u32,
    #[prost(bytes = "vec", tag = "5")]
    pub source_unit: Vec<u8>,
    #[prost(uint64, tag = "6")]
    pub source_start: u64,
    #[prost(bytes = "vec", tag = "7")]
    pub content: Vec<u8>,
    #[prost(string, tag = "8")]
    pub text: String,
    #[prost(uint64, tag = "9")]
    pub source_length: u64,
}

pub(super) fn encode(bodies: &DefinitionBodies) -> Bodies {
    Bodies {
        entries: bodies
            .entries
            .iter()
            .map(|body| Body {
                callable: Some(encode_callable(&body.callable)),
                access: match body.access {
                    BodyAccess::Package => 1,
                    BodyAccess::Exported => 2,
                },
                syntax_schema: body.source.schema,
                form: match body.source.form {
                    RetainedBodyForm::BlockContents => 1,
                    RetainedBodyForm::Expression => 2,
                    RetainedBodyForm::HandledBlock => 3,
                    RetainedBodyForm::HandledExpression => 4,
                },
                source_unit: body.source.source_unit.as_bytes().to_vec(),
                source_start: body.source.source_start,
                source_length: body.source.source_length,
                content: body.source.content.as_bytes().to_vec(),
                text: body.source.text.clone(),
            })
            .collect(),
    }
}

pub(super) fn decode(bodies: Bodies) -> Result<DefinitionBodies, Error> {
    Ok(DefinitionBodies {
        entries: bodies
            .entries
            .into_iter()
            .map(|body| {
                Ok(CallableBodyInfo {
                    callable: decode_callable(required(body.callable, "retained body callable")?)?,
                    access: match body.access {
                        1 => BodyAccess::Package,
                        2 => BodyAccess::Exported,
                        _ => return Err(invalid("unknown retained body access policy")),
                    },
                    source: RetainedBodySource {
                        schema: body.syntax_schema,
                        form: match body.form {
                            1 => RetainedBodyForm::BlockContents,
                            2 => RetainedBodyForm::Expression,
                            3 => RetainedBodyForm::HandledBlock,
                            4 => RetainedBodyForm::HandledExpression,
                            _ => return Err(invalid("unknown retained body syntax form")),
                        },
                        source_unit: fingerprint(body.source_unit)?,
                        source_start: body.source_start,
                        source_length: body.source_length,
                        content: fingerprint(body.content)?,
                        text: body.text,
                    },
                })
            })
            .collect::<Result<_, Error>>()?,
    })
}

pub(super) fn validate_wire(
    bodies: &Bodies,
    budget: &mut super::super::budget::MetadataGraphBudget,
) -> Result<(), Error> {
    budget.charge(bodies.entries.len(), "retained bodies")?;
    for body in &bodies.entries {
        if let Some(callable) = &body.callable {
            super::header_budget::callable(callable, budget)?;
        }
        budget.charge(7, "retained body coordinates")?;
    }
    Ok(())
}

fn fingerprint(bytes: Vec<u8>) -> Result<CodeFingerprint, Error> {
    bytes
        .try_into()
        .map(CodeFingerprint::from_bytes)
        .map_err(|_| invalid("retained source fingerprint must contain exactly 32 bytes"))
}
