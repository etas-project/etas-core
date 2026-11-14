module std.option;

import std.core.abort;

@std.role("std.option.Option")
public enum Option<T> { Some(T), None, }

@std.role("std.option.Some")
public flow Some<T>(value: T) -> Option<T> ![] { return Option.Some(value); }

@std.role("std.option.None")
public flow None<T>() -> Option<T> ![] { return Option.None; }

public flow is_some<T>(value: Option<T>) -> bool ![] {
    return match value { Option.Some(_) => true, Option.None => false };
}

public flow is_none<T>(value: Option<T>) -> bool ![] {
    return match value { Option.Some(_) => false, Option.None => true };
}

public flow unwrap<T>(value: Option<T>) -> T ![] {
    return match value { Option.Some(inner) => inner, Option.None => abort("unwrap encountered None") };
}

impl Option<T> {
    public flow is_some(self) -> bool ![] { return is_some(self); }
    public flow is_none(self) -> bool ![] { return is_none(self); }
    public flow unwrap(self) -> T ![] { return unwrap(self); }
}
