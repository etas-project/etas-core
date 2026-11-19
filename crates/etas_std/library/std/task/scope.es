module std.task;

// Enter owns a fresh execution scope and drains its children on every exit.
// The caller supplies the source callback, including its Handler installation.
type Scope = {};

// Completed tasks carry an awaitable source value, including Result.Err.
// Cancelled tasks still publish a terminal event without an awaitable value.
type TaskEvent = { task: u64, order: u64, completed: bool };

impl Scope {
    @std.role("std.task.Scope.enter")
    flow enter<T, effect E>(implementation: ? ~ std.task.scheduler.Scheduler, body: (Scheduler) -> T ![E], inherit_policy: bool) -> T ![E];

    // Combinators copy the active source policy into their fresh owned scope.
    // The fallback is an ordinary source value, used outside any active run.
    flow inherit<T, effect E>(body: (Scheduler) -> T ![E]) -> T ![E] {
        return Scope.enter(std.task.scheduler.DefaultScheduler, (scheduler: Scheduler) => {
            let adapter = std.task.scheduler.handlers(scheduler);
            return handle {
                body(scheduler)
            } with adapter;
        }, true);
    }
}

impl Scheduler {
    // Deliver the next actual completion event with atomic wait registration.
    @std.role("std.task.Scheduler.next_completion")
    flow next_completion(self) -> TaskEvent ![];

    // Request cancellation in this owner and let scope close drain the tasks.
    @std.role("std.task.Scheduler.cancel")
    flow cancel<T, effect E>(self, task: Task<T, ![E]>) -> unit ![];
}
