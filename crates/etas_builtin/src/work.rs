#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkStopReason {
    Yield,
    OutputLimit,
    ScratchLimit,
}

#[derive(Clone, Debug)]
pub struct WorkControl {
    work_remaining: usize,
    output_remaining: usize,
    scratch_remaining: usize,
}

impl WorkControl {
    pub fn new(work: usize, output: usize, scratch: usize) -> Self {
        Self {
            work_remaining: work,
            output_remaining: output,
            scratch_remaining: scratch,
        }
    }

    pub fn work_remaining(&self) -> usize {
        self.work_remaining
    }

    pub fn consume(&mut self, units: usize) -> Result<(), WorkStopReason> {
        if units > self.work_remaining {
            return Err(WorkStopReason::Yield);
        }
        self.work_remaining -= units;
        Ok(())
    }

    pub fn reserve_output(&mut self, bytes: usize) -> Result<(), WorkStopReason> {
        if bytes > self.output_remaining {
            return Err(WorkStopReason::OutputLimit);
        }
        self.output_remaining -= bytes;
        Ok(())
    }

    pub fn reserve_scratch(&mut self, bytes: usize) -> Result<(), WorkStopReason> {
        if bytes > self.scratch_remaining {
            return Err(WorkStopReason::ScratchLimit);
        }
        self.scratch_remaining -= bytes;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkPoll<T> {
    Pending,
    Ready(T),
}
