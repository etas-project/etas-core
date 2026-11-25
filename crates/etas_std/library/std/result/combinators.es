module std.result;

impl Result<T, F> {
    public flow unwrap_or(self, default: T) -> T ![] {
        return match self { Ok(value) => value, Err(_) => default };
    }
    public flow unwrap_or_else<effect E>(self, fallback: (F) -> T ![E]) -> T ![E] {
        return match self { Ok(value) => value, Err(error) => fallback(error) };
    }
    public flow map<U, effect E>(self, f: (T) -> U ![E]) -> Result<U, F> ![E] {
        return match self { Ok(value) => Ok(f(value)), Err(error) => Err(error) };
    }
    public flow map_err<G, effect E>(self, f: (F) -> G ![E]) -> Result<T, G> ![E] {
        return match self { Ok(value) => Ok(value), Err(error) => Err(f(error)) };
    }
    public flow and_then<U, effect E>(self, f: (T) -> Result<U, F> ![E]) -> Result<U, F> ![E] {
        return match self { Ok(value) => f(value), Err(error) => Err(error) };
    }
    public flow or_else<G, effect E>(self, f: (F) -> Result<T, G> ![E]) -> Result<T, G> ![E] {
        return match self { Ok(value) => Ok(value), Err(error) => f(error) };
    }
    public flow ok(self) -> Option<T> ![] {
        return match self { Ok(value) => Some(value), Err(_) => None };
    }
    public flow err(self) -> Option<F> ![] {
        return match self { Ok(_) => None, Err(error) => Some(error) };
    }
}
impl Result<Result<T, F>, F> {
    public flow flatten(self) -> Result<T, F> ![] {
        return match self { Ok(value) => value, Err(error) => Err(error) };
    }
}
impl Result<Option<T>, F> {
    public flow transpose(self) -> Option<Result<T, F>> ![] {
        return match self {
            Err(error) => Some(Err(error)),
            Ok(value) => match value { None => None, Some(inner) => Some(Ok(inner)) },
        };
    }
}
