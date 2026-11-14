module std.result;
@std.role("std.result.Result")
public enum Result<T, E> { Ok(T), Err(E), }
public flow is_ok<T, E>(value: Result<T, E>) -> bool ![] {
    return match value { Result.Ok(_) => true, Result.Err(_) => false };
}
public flow is_err<T, E>(value: Result<T, E>) -> bool ![] {
    return match value { Result.Ok(_) => false, Result.Err(_) => true };
}

@std.role("std.result.Ok")
public flow Ok<T, E>(value: T) -> Result<T, E> ![] { return Result.Ok(value); }
@std.role("std.result.Err")
public flow Err<T, E>(error: E) -> Result<T, E> ![] { return Result.Err(error); }

impl Result<T, E> {
    public flow is_ok(self) -> bool ![] { return match self { Result.Ok(_) => true, Result.Err(_) => false }; }
    public flow is_err(self) -> bool ![] { return match self { Result.Ok(_) => false, Result.Err(_) => true }; }
}
