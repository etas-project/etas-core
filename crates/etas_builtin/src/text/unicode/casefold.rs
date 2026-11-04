use crate::{
    BuiltinError,
    text::output::TextOutputWork,
    work::{WorkControl, WorkPoll},
};

use super::data;

pub fn casefold(source: &str) -> Result<String, BuiltinError> {
    let mut output = String::new();
    output
        .try_reserve(source.len())
        .map_err(|_| BuiltinError::ResourceExhausted)?;
    for scalar in source.chars().map(u32::from) {
        if let Some(mapping) = data::casefold(scalar) {
            output
                .try_reserve(mapping.len().saturating_mul(4))
                .map_err(|_| BuiltinError::ResourceExhausted)?;
            output.extend(
                mapping
                    .iter()
                    .copied()
                    .map(|scalar| char::from_u32(scalar).expect("generated Unicode scalar")),
            );
        } else {
            output.push(char::from_u32(scalar).expect("source Unicode scalar"));
        }
    }
    Ok(output)
}

#[derive(Clone, Debug, Default)]
pub struct TextCasefoldWork {
    output: TextOutputWork,
    pending: Vec<char>,
    pending_index: usize,
}

impl TextCasefoldWork {
    pub fn reserved_bytes(&self) -> usize {
        self.output.reserved_bytes()
    }

    pub fn step(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> Result<(usize, WorkPoll<String>), BuiltinError> {
        let mut consumed = 0usize;
        loop {
            while let Some(ch) = self.pending.get(self.pending_index).copied() {
                if !self.output.append_char(ch, control)? {
                    return Ok((consumed, WorkPoll::Pending));
                }
                self.pending_index += 1;
            }
            self.pending.clear();
            self.pending_index = 0;
            let Some(ch) = input[consumed..].chars().next() else {
                return Ok((
                    consumed,
                    if end_of_input {
                        WorkPoll::Ready(self.output.take())
                    } else {
                        WorkPoll::Pending
                    },
                ));
            };
            if let Some(mapping) = data::casefold(u32::from(ch)) {
                self.pending.extend(
                    mapping
                        .iter()
                        .map(|scalar| char::from_u32(*scalar).expect("generated Unicode scalar")),
                );
            } else {
                self.pending.push(ch);
            }
            consumed += ch.len_utf8();
        }
    }
}
