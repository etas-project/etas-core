use crate::{
    BuiltinError, BuiltinValue,
    work::{WorkControl, WorkPoll, WorkStopReason},
};

use super::TextQuery;

#[derive(Clone, Debug)]
pub enum TextQueryWork {
    CharCount { count: usize },
    IsAscii,
}

impl TextQueryWork {
    pub fn new(query: TextQuery, _pattern: Option<Vec<u8>>) -> Option<Self> {
        match query {
            TextQuery::CharCount => Some(Self::CharCount { count: 0 }),
            TextQuery::IsAscii => Some(Self::IsAscii),
            TextQuery::Len
            | TextQuery::IsEmpty
            | TextQuery::Contains
            | TextQuery::StartsWith
            | TextQuery::EndsWith => None,
        }
    }

    pub fn step(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> Result<(usize, WorkPoll<BuiltinValue>), BuiltinError> {
        match self {
            Self::CharCount { count } => {
                let mut consumed = 0;
                for ch in input.chars() {
                    let bytes = ch.len_utf8();
                    if let Err(WorkStopReason::Yield) = control.consume(bytes) {
                        break;
                    }
                    *count = count.checked_add(1).ok_or(BuiltinError::NumericOverflow)?;
                    consumed += bytes;
                }
                let complete = end_of_input && consumed == input.len();
                Ok((
                    consumed,
                    if complete {
                        WorkPoll::Ready(BuiltinValue::Usize(*count))
                    } else {
                        WorkPoll::Pending
                    },
                ))
            }
            Self::IsAscii => {
                let consumed = input.len().min(control.work_remaining());
                control.consume(consumed).expect("bounded ASCII scan work");
                if !input.as_bytes()[..consumed].is_ascii() {
                    return Ok((consumed, WorkPoll::Ready(BuiltinValue::Bool(false))));
                }
                Ok((
                    consumed,
                    if end_of_input && consumed == input.len() {
                        WorkPoll::Ready(BuiltinValue::Bool(true))
                    } else {
                        WorkPoll::Pending
                    },
                ))
            }
        }
    }
}
