module std.option;

import std.core.abort;
public import std.option.Option.Some;
public import std.option.Option.None;


public enum Option<T> { Some(T), None, }



impl Option<T> {
    public flow is_some(self) -> bool ![] {
        return match self {
            Option.Some(_) => true,
            Option.None => false,
        };
    }

    public flow is_none(self) -> bool ![] {
        return match self {
            Option.Some(_) => false,
            Option.None => true,
        };
    }

    public flow unwrap(self) -> T ![] {
        return match self {
            Option.Some(inner) => inner,
            Option.None => abort("unwrap encountered None"),
        };
    }
}
