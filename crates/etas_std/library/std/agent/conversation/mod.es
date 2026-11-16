module std.agent.conversation;

import std.agent.{StepResult, ToolResult, StepDiagnostic};
import std.agent.session.Conversation;
import std.agent.prompt.Prompt;

// In-memory assembly preserves protocol data, without persistent session I/O.
@std.binding("std.agent.conversation.from_prompt")
public flow from_prompt(prompt: Prompt) -> Conversation ![] {}

@std.binding("std.agent.conversation.append_step")
public flow append_step<C, O>(conversation: Conversation, step: StepResult<C, O>) -> Result<Conversation, StepDiagnostic> ![] {}

@std.binding("std.agent.conversation.append_results")
public flow append_results<C>(conversation: Conversation, results: Array<ToolResult<C>>) -> Result<Conversation, StepDiagnostic> ![] {}
