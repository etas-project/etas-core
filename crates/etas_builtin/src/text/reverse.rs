use crate::{
    BuiltinError,
    text::{
        output::TextOutputWork,
        unicode::{GraphemeScanStep, GraphemeScanner},
    },
    work::{WorkControl, WorkPoll, WorkStopReason},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextReverseKind {
    Chars,
    Graphemes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReversePhase {
    Scan,
    Emit,
}

#[derive(Clone, Debug)]
pub struct TextReverseWork {
    kind: TextReverseKind,
    phase: ReversePhase,
    scanner: GraphemeScanner,
    source: String,
    boundaries: Vec<usize>,
    emit_segment: usize,
    emit_offset: usize,
    output: TextOutputWork,
}

impl TextReverseWork {
    pub fn new(kind: TextReverseKind) -> Self {
        Self {
            kind,
            phase: ReversePhase::Scan,
            scanner: GraphemeScanner::default(),
            source: String::new(),
            boundaries: vec![0],
            emit_segment: 0,
            emit_offset: 0,
            output: TextOutputWork::default(),
        }
    }

    pub fn reserved_output_bytes(&self) -> usize {
        self.output.reserved_bytes()
    }

    pub fn reserved_scratch_bytes(&self) -> usize {
        self.source.capacity().saturating_add(
            self.boundaries
                .capacity()
                .saturating_mul(std::mem::size_of::<usize>()),
        )
    }

    pub fn step(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> Result<(usize, WorkPoll<String>), BuiltinError> {
        let consumed = match self.phase {
            ReversePhase::Scan => match self.kind {
                TextReverseKind::Chars => self.scan_chars(input, end_of_input, control)?,
                TextReverseKind::Graphemes => self.scan_graphemes(input, end_of_input, control)?,
            },
            ReversePhase::Emit => 0,
        };
        if self.phase == ReversePhase::Scan {
            return Ok((consumed, WorkPoll::Pending));
        }
        loop {
            if self.emit_segment == 0 {
                return Ok((consumed, WorkPoll::Ready(self.output.take())));
            }
            let end = self.boundaries[self.emit_segment];
            if self.emit_offset == end {
                self.emit_segment -= 1;
                if self.emit_segment == 0 {
                    continue;
                }
                self.emit_offset = self.boundaries[self.emit_segment - 1];
                continue;
            }
            let written = self
                .output
                .append_step(&self.source[self.emit_offset..end], control)?;
            self.emit_offset += written;
            if written == 0 || control.work_remaining() == 0 {
                return Ok((consumed, WorkPoll::Pending));
            }
        }
    }

    fn scan_chars(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> Result<usize, BuiltinError> {
        let mut consumed = 0usize;
        while let Some(ch) = input[consumed..].chars().next() {
            let width = ch.len_utf8();
            if width > control.work_remaining() {
                return Ok(consumed);
            }
            reserve_scratch(control, width.saturating_add(std::mem::size_of::<usize>()))?;
            self.source
                .try_reserve_exact(width)
                .map_err(|_| BuiltinError::ResourceExhausted)?;
            self.boundaries
                .try_reserve_exact(1)
                .map_err(|_| BuiltinError::ResourceExhausted)?;
            control
                .consume(width)
                .expect("scalar width was checked against remaining work");
            self.source.push(ch);
            self.boundaries.push(self.source.len());
            consumed += width;
        }
        if end_of_input {
            self.begin_emit(control)?;
        }
        Ok(consumed)
    }

    fn scan_graphemes(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> Result<usize, BuiltinError> {
        let (consumed, poll) = self.scanner.step(input, end_of_input, control);
        if consumed != 0 {
            reserve_scratch(control, consumed)?;
            self.source
                .try_reserve_exact(consumed)
                .map_err(|_| BuiltinError::ResourceExhausted)?;
            self.source.push_str(&input[..consumed]);
        }
        match poll {
            WorkPoll::Pending => {}
            WorkPoll::Ready(GraphemeScanStep::Boundary(boundary)) => {
                if boundary != *self.boundaries.last().expect("initial boundary") {
                    reserve_scratch(control, std::mem::size_of::<usize>())?;
                    self.boundaries
                        .try_reserve_exact(1)
                        .map_err(|_| BuiltinError::ResourceExhausted)?;
                    self.boundaries.push(boundary);
                }
            }
            WorkPoll::Ready(GraphemeScanStep::End) => self.begin_emit(control)?,
        }
        Ok(consumed)
    }

    fn begin_emit(&mut self, control: &mut WorkControl) -> Result<(), BuiltinError> {
        self.output.reserve_exact(self.source.len(), control)?;
        self.emit_segment = self.boundaries.len() - 1;
        self.emit_offset = self
            .emit_segment
            .checked_sub(1)
            .map(|index| self.boundaries[index])
            .unwrap_or(0);
        self.phase = ReversePhase::Emit;
        Ok(())
    }
}

fn reserve_scratch(control: &mut WorkControl, bytes: usize) -> Result<(), BuiltinError> {
    control
        .reserve_scratch(bytes)
        .map_err(|reason| match reason {
            WorkStopReason::ScratchLimit => BuiltinError::ResourceExhausted,
            WorkStopReason::Yield | WorkStopReason::OutputLimit => BuiltinError::Abort {
                message: "invalid text reverse work limit".into(),
            },
        })
}
