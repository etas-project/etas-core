use crate::{BuiltinError, TextFailure, text::output::TextOutputWork, work::WorkControl};

#[derive(Clone, Debug, PartialEq, Eq)]
enum FormatState {
    Text,
    OpenBrace,
    CloseBrace,
    Placeholder { index: usize, digits: usize },
    Argument,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextFormatProgress {
    Pending,
    Argument(usize),
    ArgumentComplete,
    Ready(Result<String, TextFailure>),
}

#[derive(Clone, Debug)]
pub struct TextFormatWork {
    state: FormatState,
    output: TextOutputWork,
}

impl Default for TextFormatWork {
    fn default() -> Self {
        Self {
            state: FormatState::Text,
            output: TextOutputWork::default(),
        }
    }
}

impl TextFormatWork {
    pub fn reserved_output_bytes(&self) -> usize {
        self.output.reserved_bytes()
    }

    pub fn template_step(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> Result<(usize, TextFormatProgress), BuiltinError> {
        if self.state == FormatState::Argument {
            return Err(BuiltinError::Abort {
                message: "format template advanced before its argument completed".into(),
            });
        }
        let mut consumed = 0usize;
        while let Some(ch) = input[consumed..].chars().next() {
            match self.state {
                FormatState::Text => match ch {
                    '{' => {
                        if control.consume(ch.len_utf8()).is_err() {
                            return Ok((consumed, TextFormatProgress::Pending));
                        }
                        self.state = FormatState::OpenBrace;
                    }
                    '}' => {
                        if control.consume(ch.len_utf8()).is_err() {
                            return Ok((consumed, TextFormatProgress::Pending));
                        }
                        self.state = FormatState::CloseBrace;
                    }
                    _ => {
                        if !self.output.append_char(ch, control)? {
                            return Ok((consumed, TextFormatProgress::Pending));
                        }
                    }
                },
                FormatState::OpenBrace => {
                    if ch == '{' {
                        if !self.output.append_char('{', control)? {
                            return Ok((consumed, TextFormatProgress::Pending));
                        }
                        self.state = FormatState::Text;
                    } else if let Some(digit) = ch.to_digit(10) {
                        if control.consume(ch.len_utf8()).is_err() {
                            return Ok((consumed, TextFormatProgress::Pending));
                        }
                        self.state = FormatState::Placeholder {
                            index: digit as usize,
                            digits: 1,
                        };
                    } else {
                        return Ok((
                            consumed + ch.len_utf8(),
                            TextFormatProgress::Ready(Err(TextFailure::InvalidFormat)),
                        ));
                    }
                }
                FormatState::CloseBrace => {
                    if ch != '}' {
                        return Ok((
                            consumed + ch.len_utf8(),
                            TextFormatProgress::Ready(Err(TextFailure::InvalidFormat)),
                        ));
                    }
                    if !self.output.append_char('}', control)? {
                        return Ok((consumed, TextFormatProgress::Pending));
                    }
                    self.state = FormatState::Text;
                }
                FormatState::Placeholder {
                    ref mut index,
                    ref mut digits,
                } => {
                    if ch == '}' {
                        if control.consume(ch.len_utf8()).is_err() {
                            return Ok((consumed, TextFormatProgress::Pending));
                        }
                        let selected = *index;
                        self.state = FormatState::Argument;
                        consumed += ch.len_utf8();
                        return Ok((consumed, TextFormatProgress::Argument(selected)));
                    }
                    let Some(digit) = ch.to_digit(10) else {
                        return Ok((
                            consumed + ch.len_utf8(),
                            TextFormatProgress::Ready(Err(TextFailure::InvalidFormat)),
                        ));
                    };
                    if control.consume(ch.len_utf8()).is_err() {
                        return Ok((consumed, TextFormatProgress::Pending));
                    }
                    let Some(next) = index
                        .checked_mul(10)
                        .and_then(|index| index.checked_add(digit as usize))
                    else {
                        return Ok((
                            consumed + ch.len_utf8(),
                            TextFormatProgress::Ready(Err(TextFailure::InvalidFormat)),
                        ));
                    };
                    *index = next;
                    *digits += 1;
                }
                FormatState::Argument => unreachable!("checked before template scan"),
            }
            consumed += ch.len_utf8();
        }
        if !end_of_input {
            return Ok((consumed, TextFormatProgress::Pending));
        }
        Ok((
            consumed,
            if self.state == FormatState::Text {
                TextFormatProgress::Ready(Ok(self.output.take()))
            } else {
                TextFormatProgress::Ready(Err(TextFailure::InvalidFormat))
            },
        ))
    }

    pub fn argument_step(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> Result<(usize, TextFormatProgress), BuiltinError> {
        if self.state != FormatState::Argument {
            return Err(BuiltinError::Abort {
                message: "format argument advanced without a placeholder".into(),
            });
        }
        let consumed = self.output.append_step(input, control)?;
        if consumed != input.len() || !end_of_input {
            return Ok((consumed, TextFormatProgress::Pending));
        }
        self.state = FormatState::Text;
        Ok((consumed, TextFormatProgress::ArgumentComplete))
    }
}

pub fn format(
    template: &str,
    arguments: &[String],
) -> Result<Result<String, TextFailure>, BuiltinError> {
    let mut work = TextFormatWork::default();
    let mut template_offset = 0usize;
    let mut argument: Option<(usize, usize)> = None;
    loop {
        let mut control = WorkControl::new(usize::MAX, usize::MAX, usize::MAX);
        let progress = if let Some((index, offset)) = argument {
            let Some(value) = arguments.get(index) else {
                return Ok(Err(TextFailure::InvalidFormat));
            };
            let (consumed, progress) = work.argument_step(&value[offset..], true, &mut control)?;
            argument = Some((index, offset + consumed));
            progress
        } else {
            let (consumed, progress) =
                work.template_step(&template[template_offset..], true, &mut control)?;
            template_offset += consumed;
            progress
        };
        match progress {
            TextFormatProgress::Pending => {}
            TextFormatProgress::Argument(index) => {
                if index >= arguments.len() {
                    return Ok(Err(TextFailure::InvalidFormat));
                }
                argument = Some((index, 0));
            }
            TextFormatProgress::ArgumentComplete => argument = None,
            TextFormatProgress::Ready(result) => return Ok(result),
        }
    }
}
