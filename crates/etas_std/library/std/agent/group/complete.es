module std.agent.group;

public enum GroupError { EmptyParticipants, }
public enum GroupStop { PredicateSatisfied, TurnLimitReached, }
public type RoundRobinResult<S> = { state: S, turns: usize, reason: GroupStop };

public flow round_robin<S, effect E>(
    participants: Array<(S) -> S ![E]>, initial: S, max_turns: usize,
    stop_when: (S) -> bool ![],
) -> RoundRobinResult<S> ![E, Error<GroupError>] {
    let count = participants.len();
    if count == 0 { perform Error.raise<GroupError>(GroupError.EmptyParticipants); }
    var state = initial;
    var turns: usize = 0;
    if stop_when(state) {
        return RoundRobinResult { state= state, turns= turns, reason= GroupStop.PredicateSatisfied };
    }
    while turns < max_turns {
        let participant = match participants.get(turns % count) {
            Some(callback) => callback,
            None => std.core.abort("checked round-robin index"),
        };
        state = participant(state);
        turns = turns + 1;
        if stop_when(state) {
            return RoundRobinResult { state= state, turns= turns, reason= GroupStop.PredicateSatisfied };
        }
    }
    return RoundRobinResult { state= state, turns= turns, reason= GroupStop.TurnLimitReached };
}
