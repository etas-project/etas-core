module std.memory;

import std.agent.prompt.{PromptEncode, PromptPart};

impl MemorySelection<V ~ PromptEncode> ~ PromptEncode {
    public flow encode(self) -> PromptPart {
        let values = self.collect();
        return values::PromptEncode.encode();
    }
}
