module std.agent.conversation;

import std.agent.{AssistantTurn, StepDecision, StepResult, ToolRef, ToolResult, StepDiagnostic};
import std.agent.session.Conversation;
import std.agent.prompt.Prompt;
import std.json.JsonValue;
import std.collections.OrderedMap;


// These records retain protocol data only. They do not retain a checked tool
// invocation context or grant authority to execute the named tool.
type ProtocolCall = { id: string, tool: ToolRef, args: JsonValue };
type ProtocolResult = { call_id: string, result: Result<JsonValue, StepDiagnostic> };
enum ProtocolEvent {
    Assistant(AssistantTurn, Array<ProtocolCall>),
    Results(Array<ProtocolResult>),
    Instruction(Prompt),
}

public type Protocol = {
    private prompt: Prompt,
    private turns: List<ProtocolEvent>,
    private pending: Array<ProtocolCall>,
};

impl StepDiagnostic {
    flow protocol_error(message: string) -> StepDiagnostic ![] {
        return StepDiagnostic { code= "invalid_protocol", message= message, path= None };
    }
}

public flow from_prompt(prompt: Prompt) -> Conversation ![] {
    return Conversation {
        session= None,
        messages= [],
        summary= None,
        cursor= None,
        protocol_state= Some(Protocol { prompt = prompt, turns = [], pending = [] }),
        history_fence= None,
    };
}

public flow append_step<C, O>(conversation: Conversation, step: StepResult<C, O>) -> Result<Conversation, StepDiagnostic> ![] {
    let protocol_state = match conversation.protocol_state {
        Some(value) => value,
        None => Protocol { prompt= Prompt.new(), turns= [], pending= [] },
    };
    if protocol_state.pending.length != 0 {
        return Err(StepDiagnostic.protocol_error("append all pending tool results before another assistant turn"));
    }
    var pending: Array<ProtocolCall> = [];
    match step.decision {
        StepDecision.CallTools(calls) => {
            if calls.length == 0 {
                return Err(StepDiagnostic.protocol_error("a tool request turn must contain at least one call"));
            }
            var seen: OrderedMap<string, bool> = OrderedMap.new();
            for call in calls {
                if call.id == "" || seen.contains_key(call.id) {
                    return Err(StepDiagnostic.protocol_error("tool call IDs must be nonempty and unique within a turn"));
                }
                seen = seen.insert(call.id, true);
                pending = pending.push(ProtocolCall { id= call.id, tool= call.tool, args= call.args });
            }
        },
        StepDecision.Final(_) => {},
        StepDecision.Stopped(_) => {},
    };
    var output = conversation;
    output.protocol_state = Some(Protocol {
        prompt = protocol_state.prompt,
        turns = ProtocolEvent.Assistant(step.turn, pending) :: protocol_state.turns,
        pending = pending,
    });
    return Ok(output);
}

public flow append_results<C>(conversation: Conversation, results: Array<ToolResult<C>>) -> Result<Conversation, StepDiagnostic> ![] {
    let protocol_state = match conversation.protocol_state {
        Some(value) => value,
        None => { return Err(StepDiagnostic.protocol_error("conversation has no pending tool results")); },
    };
    if protocol_state.pending.length == 0 {
        return Err(StepDiagnostic.protocol_error("conversation has no pending tool results"));
    }
    if protocol_state.pending.length != results.length {
        return Err(StepDiagnostic.protocol_error("tool results must complete exactly the pending call set"));
    }
    var by_id: OrderedMap<string, ProtocolResult> = OrderedMap.new();
    for result in results {
        if by_id.contains_key(result.call_id) {
            return Err(StepDiagnostic.protocol_error("tool results contain duplicate call IDs"));
        }
        by_id = by_id.insert(result.call_id, ProtocolResult { call_id= result.call_id, result= result.result });
    }
    var ordered: Array<ProtocolResult> = [];
    for call in protocol_state.pending {
        match by_id.get(call.id) {
            None => { return Err(StepDiagnostic.protocol_error("tool results contain a missing or foreign call ID")); },
            Some(result) => { ordered = ordered.push(result); },
        };
    }
    var output = conversation;
    output.protocol_state = Some(Protocol {
        prompt = protocol_state.prompt,
        turns = ProtocolEvent.Results(ordered) :: protocol_state.turns,
        pending = [],
    });
    return Ok(output);
}

impl Protocol {
    // Agent method prompt composition uses the same in-memory protocol state.
    flow append_instruction(self, instruction: Prompt) -> Result<Protocol, StepDiagnostic> ![] {
        if self.pending.length != 0 {
            return Err(StepDiagnostic.protocol_error("append tool results before a prompt instruction"));
        }
        return Ok(Protocol {
            prompt= self.prompt,
            turns= ProtocolEvent.Instruction(instruction) :: self.turns,
            pending= [],
        });
    }
}
