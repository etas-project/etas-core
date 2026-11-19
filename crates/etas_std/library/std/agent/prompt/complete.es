module std.agent.prompt;

import std.security.trust.{Trusted, Public};


public spec PromptEncode {
    flow encode(self) -> PromptPart ![];
}

public type PromptPart = { text: string };

public enum PromptChannel {
    System(Trusted<string>),
    User(Public<string>),
    Assistant(string),
    Data(PromptPart),
}

public type Prompt = { parts: Array<PromptChannel> };

impl Prompt {
    public flow new() -> Prompt ![] {
        return Prompt { parts= [] };
    }

    public flow assistant(self, content: string) -> Prompt ![] {
        return Prompt { parts= self.parts.push(PromptChannel.Assistant(content)) };
    }

    public flow data<T ~ PromptEncode>(self, content: T) -> Prompt ![] {
        let part = content::PromptEncode.encode();
        return Prompt { parts= self.parts.push(PromptChannel.Data(part)) };
    }

    public flow system(self, content: Trusted<string>) -> Prompt ![] {
        return Prompt { parts= self.parts.push(PromptChannel.System(content)) };
    }

    public flow user(self, content: Public<string>) -> Prompt ![] {
        return Prompt { parts= self.parts.push(PromptChannel.User(content)) };
    }
}

impl PromptPart {
    public flow data<T ~ PromptEncode>(content: T) -> PromptPart ![] {
        let encoded = content::PromptEncode.encode();
        return PromptPart { text= encoded.text };
    }
}
