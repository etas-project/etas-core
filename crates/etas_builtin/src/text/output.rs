use crate::{
    BuiltinError,
    work::{WorkControl, WorkStopReason},
};

#[derive(Clone, Debug, Default)]
pub struct TextOutputWork {
    output: String,
    reserved_bytes: usize,
}

impl TextOutputWork {
    pub fn reserved_bytes(&self) -> usize {
        self.reserved_bytes
    }

    pub fn len(&self) -> usize {
        self.output.len()
    }

    pub fn is_empty(&self) -> bool {
        self.output.is_empty()
    }

    pub fn reserve_exact(
        &mut self,
        total: usize,
        control: &mut WorkControl,
    ) -> Result<(), BuiltinError> {
        if total > isize::MAX as usize {
            return Err(BuiltinError::Text(crate::TextFailure::LengthOverflow));
        }
        self.reserve_to(total, control)
    }

    pub fn append_step(
        &mut self,
        input: &str,
        control: &mut WorkControl,
    ) -> Result<usize, BuiltinError> {
        if input.is_empty() || control.work_remaining() == 0 {
            return Ok(0);
        }
        let mut consumed = input.len().min(control.work_remaining());
        while consumed != 0 && !input.is_char_boundary(consumed) {
            consumed -= 1;
        }
        if consumed == 0 {
            return Ok(0);
        }
        let total = self
            .output
            .len()
            .checked_add(consumed)
            .ok_or(BuiltinError::Text(crate::TextFailure::LengthOverflow))?;
        self.reserve_to(total, control)?;
        self.output.push_str(&input[..consumed]);
        control
            .consume(consumed)
            .expect("text output append is bounded by remaining work");
        Ok(consumed)
    }

    pub fn append_char(
        &mut self,
        value: char,
        control: &mut WorkControl,
    ) -> Result<bool, BuiltinError> {
        if value.len_utf8() > control.work_remaining() {
            return Ok(false);
        }
        let total = self
            .output
            .len()
            .checked_add(value.len_utf8())
            .ok_or(BuiltinError::Text(crate::TextFailure::LengthOverflow))?;
        self.reserve_to(total, control)?;
        self.output.push(value);
        control
            .consume(value.len_utf8())
            .expect("text output scalar is bounded by remaining work");
        Ok(true)
    }

    pub fn take(&mut self) -> String {
        self.reserved_bytes = 0;
        std::mem::take(&mut self.output)
    }

    pub fn replace_same_width(&mut self, offset: usize, before: &str, after: &str) {
        debug_assert_eq!(before.len(), after.len());
        debug_assert_eq!(&self.output[offset..offset + before.len()], before);
        self.output
            .replace_range(offset..offset + before.len(), after);
    }

    fn reserve_to(&mut self, total: usize, control: &mut WorkControl) -> Result<(), BuiltinError> {
        if total <= self.reserved_bytes {
            return Ok(());
        }
        let additional = total - self.reserved_bytes;
        control
            .reserve_output(additional)
            .map_err(output_limit_error)?;
        self.output
            .try_reserve_exact(additional)
            .map_err(|_| BuiltinError::ResourceExhausted)?;
        self.reserved_bytes = total;
        Ok(())
    }
}

fn output_limit_error(reason: WorkStopReason) -> BuiltinError {
    match reason {
        WorkStopReason::OutputLimit => BuiltinError::ResourceExhausted,
        WorkStopReason::Yield | WorkStopReason::ScratchLimit => BuiltinError::Abort {
            message: "invalid text output work limit".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_only_complete_utf8_scalars_and_honors_output_limit() {
        let mut output = TextOutputWork::default();
        let mut first = WorkControl::new(2, 8, 0);
        assert_eq!(output.append_step("😀x", &mut first).unwrap(), 0);
        let mut second = WorkControl::new(4, 4, 0);
        assert_eq!(output.append_step("😀x", &mut second).unwrap(), 4);
        let mut denied = WorkControl::new(1, 0, 0);
        assert_eq!(
            output.append_step("x", &mut denied),
            Err(BuiltinError::ResourceExhausted)
        );
        assert_eq!(output.take(), "😀");
    }
}
