module std.agent.prompt;

import std.json.{JsonValue, stringify};


impl PromptPart ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        return PromptPart { text= self.text };
    }
}

impl string ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        let text = match stringify(JsonValue.String(self)) {
            Ok(encoded) => encoded,
            Err(_) => std.core.abort("string prompt encoding failed"),
        };
        return PromptPart { text= text };
    }
}

impl bool ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        return PromptPart { text= if self { "true" } else { "false" } };
    }
}

impl unit ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        return PromptPart { text= "null" };
    }
}

impl i8 ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        let text = std.text.from_i8(self);
        return PromptPart { text= text };
    }
}

impl i16 ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        let text = std.text.from_i16(self);
        return PromptPart { text= text };
    }
}

impl i32 ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        let text = std.text.from_i32(self);
        return PromptPart { text= text };
    }
}

impl i64 ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        let text = std.text.from_i64(self);
        return PromptPart { text= text };
    }
}

impl i128 ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        if self < -9223372036854775808 || self > 9223372036854775807 {
            std.core.abort("signed integer is outside JSON i64 range");
        }
        let text = std.text.from_i128(self);
        return PromptPart { text= text };
    }
}

impl isize ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        let text = std.text.from_isize(self);
        return PromptPart { text= text };
    }
}

impl u8 ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        let text = std.text.from_u8(self);
        return PromptPart { text= text };
    }
}

impl u16 ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        let text = std.text.from_u16(self);
        return PromptPart { text= text };
    }
}

impl u32 ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        let text = std.text.from_u32(self);
        return PromptPart { text= text };
    }
}

impl u64 ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        let text = std.text.from_u64(self);
        return PromptPart { text= text };
    }
}

impl u128 ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        if self > 18446744073709551615 {
            std.core.abort("unsigned integer is outside JSON u64 range");
        }
        let text = std.text.from_u128(self);
        return PromptPart { text= text };
    }
}

impl usize ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        let text = std.text.from_usize(self);
        return PromptPart { text= text };
    }
}

impl f32 ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        if self.bits % 2147483648 >= 2139095040 {
            std.core.abort("floating-point value is not finite");
        }
        let text = std.text.from_f32(self);
        return PromptPart { text= text };
    }
}

impl f64 ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        if self.bits % 9223372036854775808 >= 9218868437227405312 {
            std.core.abort("floating-point value is not finite");
        }
        let text = std.text.from_f64(self);
        return PromptPart { text= text };
    }
}

impl Array<T ~ PromptEncode> ~ PromptEncode {
    public flow encode(self) -> PromptPart {
        var text = "[";
        var separator = "";
        for value in self {
            let part = value::PromptEncode.encode();
            text = text + separator + part.text;
            separator = ",";
        }
        return PromptPart { text= text + "]" };
    }
}

impl List<T ~ PromptEncode> ~ PromptEncode {
    public flow encode(self) -> PromptPart {
        var text = "[";
        var separator = "";
        for value in self {
            let part = value::PromptEncode.encode();
            text = text + separator + part.text;
            separator = ",";
        }
        return PromptPart { text= text + "]" };
    }
}

impl Set<T ~ PromptEncode> ~ PromptEncode {
    public flow encode(self) -> PromptPart {
        var text = "[";
        var separator = "";
        for value in self {
            let part = value::PromptEncode.encode();
            text = text + separator + part.text;
            separator = ",";
        }
        return PromptPart { text= text + "]" };
    }
}

impl Slice<T ~ PromptEncode> ~ PromptEncode {
    public flow encode(self) -> PromptPart {
        var text = "[";
        var separator = "";
        for value in self {
            let part = value::PromptEncode.encode();
            text = text + separator + part.text;
            separator = ",";
        }
        return PromptPart { text= text + "]" };
    }
}

impl bytes ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        var text = "[";
        var separator = "";
        for value in self {
            text = text + separator + std.text.from_u8(value);
            separator = ",";
        }
        return PromptPart { text= text + "]" };
    }
}

impl Map<K ~ PromptEncode, V ~ PromptEncode> ~ PromptEncode {
    public flow encode(self) -> PromptPart {
        var text = "[";
        var separator = "";
        for (key, value) in self {
            let encoded_key = key::PromptEncode.encode();
            let encoded_value = value::PromptEncode.encode();
            text = text + separator + "[" + encoded_key.text + "," + encoded_value.text + "]";
            separator = ",";
        }
        return PromptPart { text= text + "]" };
    }
}

impl Option<T ~ PromptEncode> ~ PromptEncode {
    public flow encode(self) -> PromptPart {
        return match self {
            None => PromptPart { text= "{\"fields\":[],\"name\":\"None\"}" },
            Some(value) => {
                let part = value::PromptEncode.encode();
                PromptPart { text= "{\"fields\":[" + part.text + "],\"name\":\"Some\"}" }
            },
        };
    }
}

impl JsonValue ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        let text = match stringify(self) {
            Ok(encoded) => encoded,
            Err(_) => std.core.abort("JSON prompt encoding requires finite numbers"),
        };
        return PromptPart { text= text };
    }
}

impl std.security.trust.Trusted<T ~ PromptEncode> ~ PromptEncode {
    public flow encode(self) -> PromptPart {
        let part = self.value::PromptEncode.encode();
        return PromptPart { text= part.text };
    }
}

impl std.security.trust.Public<T ~ PromptEncode> ~ PromptEncode {
    public flow encode(self) -> PromptPart {
        let part = self.value::PromptEncode.encode();
        return PromptPart { text= part.text };
    }
}

impl std.security.trust.Untrusted<T ~ PromptEncode> ~ PromptEncode {
    public flow encode(self) -> PromptPart {
        let part = self.value::PromptEncode.encode();
        return PromptPart { text= part.text };
    }
}

impl std.security.trust.Sanitized<T ~ PromptEncode> ~ PromptEncode {
    public flow encode(self) -> PromptPart {
        let part = self.value::PromptEncode.encode();
        return PromptPart { text= part.text };
    }
}
