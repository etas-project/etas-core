module std.task;

// Queue registration and continuation transfer are VM operations. The handler
// adapter and ready-selection policy remain ordinary Etas implementations.
impl Scheduler {
    @std.role("std.task.Scheduler.requeue")
    public flow requeue<T>(self, continuation: Continuation<T>, value: T) -> never ![];

    @std.role("std.task.Scheduler.park")
    public flow park<T, effect E>(self, task: Task<T, ![E]>, continuation: Continuation<T>) -> never ![];
}
