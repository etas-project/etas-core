use crate::{
    BuiltinError, BuiltinTypeTag, BuiltinValue,
    text::output::TextOutputWork,
    work::{WorkControl, WorkPoll, WorkStopReason},
};

use super::data;

const S_BASE: u32 = 0xAC00;
const L_BASE: u32 = 0x1100;
const V_BASE: u32 = 0x1161;
const T_BASE: u32 = 0x11A7;
const L_COUNT: u32 = 19;
const V_COUNT: u32 = 21;
const T_COUNT: u32 = 28;
const N_COUNT: u32 = V_COUNT * T_COUNT;
const S_COUNT: u32 = L_COUNT * N_COUNT;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalizationForm {
    Nfc,
    Nfd,
    Nfkc,
    Nfkd,
}

impl NormalizationForm {
    fn compatibility(self) -> bool {
        matches!(self, Self::Nfkc | Self::Nfkd)
    }

    fn compose(self) -> bool {
        matches!(self, Self::Nfc | Self::Nfkc)
    }

    pub fn from_builtin(value: &BuiltinValue) -> Result<Self, BuiltinError> {
        let BuiltinValue::Variant { name, fields } = value else {
            return Err(BuiltinError::TypeMismatch {
                expected: BuiltinTypeTag::Variant,
                actual: value.type_tag(),
            });
        };
        if !fields.is_empty() {
            return Err(BuiltinError::ArityMismatch {
                expected: 0,
                actual: fields.len(),
            });
        }
        match name.as_str() {
            "NFC" => Ok(Self::Nfc),
            "NFD" => Ok(Self::Nfd),
            "NFKC" => Ok(Self::Nfkc),
            "NFKD" => Ok(Self::Nfkd),
            _ => Err(BuiltinError::TypeMismatch {
                expected: BuiltinTypeTag::Variant,
                actual: value.type_tag(),
            }),
        }
    }
}

#[derive(Clone, Debug)]
enum NormalizePhase {
    Decompose,
    Order,
    OrderFlush {
        class: usize,
        index: usize,
        pending_starter: Option<u32>,
        end_of_input: bool,
    },
    Compose,
    Emit,
}

#[derive(Clone, Debug)]
pub struct TextNormalizeWork {
    form: NormalizationForm,
    phase: NormalizePhase,
    decomposition_stack: Vec<u32>,
    decomposed: Vec<u32>,
    order_index: usize,
    mark_buckets: Vec<Vec<u32>>,
    mark_count: usize,
    ordered: Vec<u32>,
    compose_index: usize,
    composed: Vec<u32>,
    starter: Option<usize>,
    last_class: u8,
    emit_index: usize,
    output: TextOutputWork,
}

impl TextNormalizeWork {
    pub fn new(form: NormalizationForm) -> Self {
        Self {
            form,
            phase: NormalizePhase::Decompose,
            decomposition_stack: Vec::new(),
            decomposed: Vec::new(),
            order_index: 0,
            mark_buckets: (0..=u8::MAX).map(|_| Vec::new()).collect(),
            mark_count: 0,
            ordered: Vec::new(),
            compose_index: 0,
            composed: Vec::new(),
            starter: None,
            last_class: 0,
            emit_index: 0,
            output: TextOutputWork::default(),
        }
    }

    pub fn reserved_output_bytes(&self) -> usize {
        self.output.reserved_bytes()
    }

    pub fn reserved_scratch_bytes(&self) -> usize {
        let scalar = std::mem::size_of::<u32>();
        (self.decomposition_stack.capacity()
            + self.decomposed.capacity()
            + self.ordered.capacity()
            + self.composed.capacity()
            + self.mark_buckets.iter().map(Vec::capacity).sum::<usize>())
        .saturating_mul(scalar)
        .saturating_add(
            self.mark_buckets
                .capacity()
                .saturating_mul(std::mem::size_of::<Vec<u32>>()),
        )
    }

    pub fn step(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> Result<(usize, WorkPoll<String>), BuiltinError> {
        let mut consumed = 0usize;
        loop {
            match self.phase.clone() {
                NormalizePhase::Decompose => {
                    if let Some(scalar) = self.decomposition_stack.pop() {
                        if control.consume(1) == Err(WorkStopReason::Yield) {
                            self.decomposition_stack.push(scalar);
                            return Ok((consumed, WorkPoll::Pending));
                        }
                        if let Some(parts) = decompose_hangul(scalar) {
                            for part in parts.into_iter().rev() {
                                push_scalar(&mut self.decomposition_stack, part, control)?;
                            }
                        } else if let Some(parts) =
                            data::decomposition(scalar, self.form.compatibility())
                        {
                            for part in parts.iter().rev().copied() {
                                push_scalar(&mut self.decomposition_stack, part, control)?;
                            }
                        } else {
                            push_scalar(&mut self.decomposed, scalar, control)?;
                        }
                        continue;
                    }
                    if let Some(ch) = input[consumed..].chars().next() {
                        push_scalar(&mut self.decomposition_stack, u32::from(ch), control)?;
                        consumed += ch.len_utf8();
                        continue;
                    }
                    if !end_of_input {
                        return Ok((consumed, WorkPoll::Pending));
                    }
                    self.phase = NormalizePhase::Order;
                }
                NormalizePhase::Order => {
                    if let Some(scalar) = self.decomposed.get(self.order_index).copied() {
                        if control.consume(1) == Err(WorkStopReason::Yield) {
                            return Ok((consumed, WorkPoll::Pending));
                        }
                        self.order_index += 1;
                        let class = data::combining_class(scalar);
                        if class == 0 {
                            if self.mark_count == 0 {
                                push_scalar(&mut self.ordered, scalar, control)?;
                            } else {
                                self.phase = NormalizePhase::OrderFlush {
                                    class: 1,
                                    index: 0,
                                    pending_starter: Some(scalar),
                                    end_of_input: false,
                                };
                            }
                        } else {
                            push_scalar(
                                &mut self.mark_buckets[usize::from(class)],
                                scalar,
                                control,
                            )?;
                            self.mark_count += 1;
                        }
                        continue;
                    }
                    self.decomposed = Vec::new();
                    if self.mark_count == 0 {
                        self.begin_compose_or_emit();
                    } else {
                        self.phase = NormalizePhase::OrderFlush {
                            class: 1,
                            index: 0,
                            pending_starter: None,
                            end_of_input: true,
                        };
                    }
                }
                NormalizePhase::OrderFlush {
                    mut class,
                    mut index,
                    pending_starter,
                    end_of_input,
                } => {
                    while class < self.mark_buckets.len() {
                        if let Some(scalar) = self.mark_buckets[class].get(index).copied() {
                            if control.consume(1) == Err(WorkStopReason::Yield) {
                                self.phase = NormalizePhase::OrderFlush {
                                    class,
                                    index,
                                    pending_starter,
                                    end_of_input,
                                };
                                return Ok((consumed, WorkPoll::Pending));
                            }
                            push_scalar(&mut self.ordered, scalar, control)?;
                            index += 1;
                            self.mark_count -= 1;
                            continue;
                        }
                        self.mark_buckets[class] = Vec::new();
                        class += 1;
                        index = 0;
                    }
                    if let Some(starter) = pending_starter {
                        push_scalar(&mut self.ordered, starter, control)?;
                    }
                    if end_of_input {
                        self.begin_compose_or_emit();
                    } else {
                        self.phase = NormalizePhase::Order;
                    }
                }
                NormalizePhase::Compose => {
                    let Some(scalar) = self.ordered.get(self.compose_index).copied() else {
                        self.ordered = Vec::new();
                        self.phase = NormalizePhase::Emit;
                        continue;
                    };
                    if control.consume(1) == Err(WorkStopReason::Yield) {
                        return Ok((consumed, WorkPoll::Pending));
                    }
                    self.compose_index += 1;
                    let class = data::combining_class(scalar);
                    if let Some(starter_index) = self.starter
                        && (self.last_class == 0 || self.last_class < class)
                        && let Some(composite) = compose_pair(self.composed[starter_index], scalar)
                    {
                        self.composed[starter_index] = composite;
                        continue;
                    }
                    push_scalar(&mut self.composed, scalar, control)?;
                    if class == 0 {
                        self.starter = Some(self.composed.len() - 1);
                    }
                    self.last_class = class;
                }
                NormalizePhase::Emit => {
                    let Some(scalar) = self.composed.get(self.emit_index).copied() else {
                        return Ok((consumed, WorkPoll::Ready(self.output.take())));
                    };
                    let ch = char::from_u32(scalar).expect("generated Unicode scalar");
                    if !self.output.append_char(ch, control)? {
                        return Ok((consumed, WorkPoll::Pending));
                    }
                    self.emit_index += 1;
                }
            }
        }
    }

    fn begin_compose_or_emit(&mut self) {
        if self.form.compose() {
            self.phase = NormalizePhase::Compose;
        } else {
            self.composed = std::mem::take(&mut self.ordered);
            self.phase = NormalizePhase::Emit;
        }
    }
}

fn push_scalar(
    values: &mut Vec<u32>,
    scalar: u32,
    control: &mut WorkControl,
) -> Result<(), BuiltinError> {
    if values.len() == values.capacity() {
        control
            .reserve_scratch(std::mem::size_of::<u32>())
            .map_err(scratch_limit_error)?;
        values
            .try_reserve_exact(1)
            .map_err(|_| BuiltinError::ResourceExhausted)?;
    }
    values.push(scalar);
    Ok(())
}

fn scratch_limit_error(reason: WorkStopReason) -> BuiltinError {
    match reason {
        WorkStopReason::ScratchLimit => BuiltinError::ResourceExhausted,
        WorkStopReason::Yield | WorkStopReason::OutputLimit => BuiltinError::Abort {
            message: "invalid Unicode normalization work limit".into(),
        },
    }
}

pub fn normalize(source: &str, form: NormalizationForm) -> Result<String, BuiltinError> {
    let mut decomposed = Vec::new();
    decomposed
        .try_reserve(source.chars().count())
        .map_err(|_| BuiltinError::ResourceExhausted)?;
    for scalar in source.chars().map(u32::from) {
        decompose_scalar(scalar, form.compatibility(), &mut decomposed)?;
    }
    canonical_order(&mut decomposed);
    if form.compose() {
        compose(&mut decomposed);
    }
    let mut output = String::new();
    output
        .try_reserve(source.len())
        .map_err(|_| BuiltinError::ResourceExhausted)?;
    output.extend(
        decomposed
            .into_iter()
            .map(|scalar| char::from_u32(scalar).expect("generated Unicode scalar")),
    );
    Ok(output)
}

pub fn is_normalized(source: &str, form: NormalizationForm) -> Result<bool, BuiltinError> {
    normalize(source, form).map(|normalized| normalized == source)
}

fn decompose_scalar(
    scalar: u32,
    compatibility: bool,
    output: &mut Vec<u32>,
) -> Result<(), BuiltinError> {
    let mut stack = vec![scalar];
    while let Some(scalar) = stack.pop() {
        if let Some(parts) = decompose_hangul(scalar) {
            stack.extend(parts.into_iter().rev());
        } else if let Some(parts) = data::decomposition(scalar, compatibility) {
            stack
                .try_reserve(parts.len())
                .map_err(|_| BuiltinError::ResourceExhausted)?;
            stack.extend(parts.iter().rev().copied());
        } else {
            output
                .try_reserve(1)
                .map_err(|_| BuiltinError::ResourceExhausted)?;
            output.push(scalar);
        }
    }
    Ok(())
}

fn decompose_hangul(scalar: u32) -> Option<Vec<u32>> {
    let index = scalar.checked_sub(S_BASE)?;
    if index >= S_COUNT {
        return None;
    }
    let l = L_BASE + index / N_COUNT;
    let v = V_BASE + (index % N_COUNT) / T_COUNT;
    let t = index % T_COUNT;
    Some(if t == 0 {
        vec![l, v]
    } else {
        vec![l, v, T_BASE + t]
    })
}

fn canonical_order(scalars: &mut [u32]) {
    let mut mark_start = 0;
    for index in 0..scalars.len() {
        if data::combining_class(scalars[index]) == 0 {
            scalars[mark_start..index].sort_by_key(|scalar| data::combining_class(*scalar));
            mark_start = index + 1;
        }
    }
    scalars[mark_start..].sort_by_key(|scalar| data::combining_class(*scalar));
}

fn compose(scalars: &mut Vec<u32>) {
    let mut result = Vec::with_capacity(scalars.len());
    let mut starter = None;
    let mut last_class = 0;
    for scalar in scalars.iter().copied() {
        let class = data::combining_class(scalar);
        if let Some(starter_index) = starter
            && (last_class == 0 || last_class < class)
            && let Some(composite) = compose_pair(result[starter_index], scalar)
        {
            result[starter_index] = composite;
            continue;
        }
        result.push(scalar);
        if class == 0 {
            starter = Some(result.len() - 1);
        }
        last_class = class;
    }
    *scalars = result;
}

fn compose_pair(starter: u32, scalar: u32) -> Option<u32> {
    if (L_BASE..L_BASE + L_COUNT).contains(&starter) && (V_BASE..V_BASE + V_COUNT).contains(&scalar)
    {
        return Some(S_BASE + ((starter - L_BASE) * V_COUNT + (scalar - V_BASE)) * T_COUNT);
    }
    if let Some(hangul) = starter.checked_sub(S_BASE)
        && hangul < S_COUNT
        && hangul % T_COUNT == 0
        && (T_BASE + 1..T_BASE + T_COUNT).contains(&scalar)
    {
        return Some(starter + scalar - T_BASE);
    }
    data::composition(starter, scalar)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resumable_normalization_matches_the_canonical_kernel_across_chunks() {
        let chunks = ["\u{212b}", "\u{301}가", "각\u{fb03}"];
        let source = chunks.concat();
        for form in [
            NormalizationForm::Nfc,
            NormalizationForm::Nfd,
            NormalizationForm::Nfkc,
            NormalizationForm::Nfkd,
        ] {
            let mut work = TextNormalizeWork::new(form);
            let mut output = None;
            for (index, chunk) in chunks.iter().enumerate() {
                let mut offset = 0;
                while offset < chunk.len() || (chunk.is_empty() && offset == 0) {
                    let mut control = WorkControl::new(4, usize::MAX, usize::MAX);
                    let (consumed, poll) = work
                        .step(&chunk[offset..], index + 1 == chunks.len(), &mut control)
                        .unwrap();
                    offset += consumed;
                    if let WorkPoll::Ready(value) = poll {
                        output = Some(value);
                        break;
                    }
                    if offset == chunk.len() {
                        break;
                    }
                }
            }
            while output.is_none() {
                let mut control = WorkControl::new(4, usize::MAX, usize::MAX);
                let (_, poll) = work.step("", true, &mut control).unwrap();
                if let WorkPoll::Ready(value) = poll {
                    output = Some(value);
                }
            }
            assert_eq!(output.unwrap(), normalize(&source, form).unwrap());
        }
    }
}
