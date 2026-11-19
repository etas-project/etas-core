module std.result;

import std.core.abort;
public import std.result.Result.Ok;
public import std.result.Result.Err;
public enum Result<T, E> { Ok(T), Err(E), }

impl Result<T, E> {
    public flow is_ok(self) -> bool ![] {
        return match self {
            Result.Ok(_) => true,
            Result.Err(_) => false,
        };
    }

    public flow is_err(self) -> bool ![] {
        return match self {
            Result.Ok(_) => false,
            Result.Err(_) => true,
        };
    }

    public flow unwrap(self) -> T ![] {
        return match self {
            Result.Ok(inner) => inner,
            Result.Err(_) => abort("unwrap encountered Err"),
        };
    }
}
