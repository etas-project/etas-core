use crate::CodeFingerprint;

use super::SyntaxIdentity;

pub const RETAINED_BODY_SOURCE_SCHEMA: u32 = 1;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum RetainedBodyForm {
    BlockContents,
    Expression,
    HandledBlock,
    HandledExpression,
}

/// Inert source bytes and producer coordinates, not a parsed syntax lease,
/// lexical environment, checked artifact, or permission to execute.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct RetainedBodySource {
    pub schema: u32,
    pub form: RetainedBodyForm,
    pub source_unit: CodeFingerprint,
    pub source_length: u64,
    pub source_start: u64,
    pub content: CodeFingerprint,
    pub text: String,
}

impl RetainedBodySource {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != RETAINED_BODY_SOURCE_SCHEMA {
            return Err("unsupported retained body syntax schema");
        }
        let end = self
            .source_start
            .checked_add(self.text.len() as u64)
            .ok_or("retained body source range overflows")?;
        if self.source_length > u32::MAX as u64 || end > self.source_length {
            return Err("retained body range exceeds its source unit");
        }
        if CodeFingerprint::of_bytes("etas.code.body-content.v1", self.text.as_bytes())
            != self.content
        {
            return Err("retained body content fingerprint mismatch");
        }
        Ok(())
    }

    pub fn identity(&self, owner: CodeFingerprint) -> Result<SyntaxIdentity, &'static str> {
        self.validate()?;
        identity(
            owner,
            self.form,
            self.source_unit,
            self.source_length,
            self.source_start,
            self.content,
            self.text.len(),
        )
    }

    /// Native views can fingerprint borrowed source without allocating a body
    /// envelope merely to compute its identity.
    pub fn identity_for_text(
        owner: CodeFingerprint,
        form: RetainedBodyForm,
        source_unit: CodeFingerprint,
        source_length: u64,
        source_start: u64,
        text: &str,
    ) -> Result<SyntaxIdentity, &'static str> {
        identity(
            owner,
            form,
            source_unit,
            source_length,
            source_start,
            CodeFingerprint::of_bytes("etas.code.body-content.v1", text.as_bytes()),
            text.len(),
        )
    }
}

fn identity(
    owner: CodeFingerprint,
    form: RetainedBodyForm,
    source_unit: CodeFingerprint,
    source_length: u64,
    source_start: u64,
    content: CodeFingerprint,
    length: usize,
) -> Result<SyntaxIdentity, &'static str> {
    let end = source_start
        .checked_add(length as u64)
        .ok_or("retained body source range overflows")?;
    if source_length > u32::MAX as u64 || end > source_length {
        return Err("retained body range exceeds its source unit");
    }
    let mut identity = Vec::with_capacity(93);
    identity.extend_from_slice(owner.as_bytes());
    identity.extend_from_slice(source_unit.as_bytes());
    identity.extend_from_slice(&source_length.to_le_bytes());
    identity.extend_from_slice(&source_start.to_le_bytes());
    identity.extend_from_slice(&end.to_le_bytes());
    identity.extend_from_slice(&RETAINED_BODY_SOURCE_SCHEMA.to_le_bytes());
    identity.push(match form {
        RetainedBodyForm::BlockContents => 1,
        RetainedBodyForm::Expression => 2,
        RetainedBodyForm::HandledBlock => 3,
        RetainedBodyForm::HandledExpression => 4,
    });
    Ok(SyntaxIdentity::new(
        CodeFingerprint::of_bytes("etas.code.retained-body-source.v2", &identity),
        content,
    ))
}
