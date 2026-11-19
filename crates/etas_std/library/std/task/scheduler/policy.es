module std.task.scheduler;

public alias SchedulerHandle = std.task.Scheduler;

public type ReadyTask = { id: u64, enqueue_order: u64 };

public spec Scheduler {
    flow select(var self, ready: Array<ReadyTask>) -> usize ![];
}

public type FifoScheduler = {};

impl FifoScheduler ~ Scheduler {
    public flow select(var self, ready: Array<ReadyTask>) -> usize ![] {
        return 0;
    }
}

public let DefaultScheduler = FifoScheduler {};
