module std.option;

impl Option<T> {
    public flow unwrap_or(self, default: T) -> T ![] {
        return match self { Some(value) => value, None => default };
    }
    public flow unwrap_or_else<effect E>(self, fallback: () -> T ![E]) -> T ![E] {
        return match self { Some(value) => value, None => fallback() };
    }
    public flow map<U, effect E>(self, f: (T) -> U ![E]) -> Option<U> ![E] {
        return match self { Some(value) => Some(f(value)), None => None };
    }
    public flow and_then<U, effect E>(self, f: (T) -> Option<U> ![E]) -> Option<U> ![E] {
        return match self { Some(value) => f(value), None => None };
    }
    public flow filter<effect E>(self, predicate: (T) -> bool ![E]) -> Option<T> ![E] {
        return match self {
            Some(value) => if predicate(value) { Some(value) } else { None },
            None => None,
        };
    }
    public flow or_else<effect E>(self, fallback: () -> Option<T> ![E]) -> Option<T> ![E] {
        return match self { Some(_) => self, None => fallback() };
    }
    public flow ok_or<F>(self, error: F) -> Result<T, F> ![] {
        return match self { Some(value) => Ok(value), None => Err(error) };
    }
    public flow ok_or_else<F, effect E>(self, error: () -> F ![E]) -> Result<T, F> ![E] {
        return match self { Some(value) => Ok(value), None => Err(error()) };
    }
    public flow zip<U>(self, other: Option<U>) -> Option<(T, U)> ![] {
        return match self {
            Some(left) => match other { Some(right) => Some((left, right)), None => None },
            None => None,
        };
    }
}
impl Option<Option<T>> {
    public flow flatten(self) -> Option<T> ![] {
        return match self { Some(value) => value, None => None };
    }
}
impl Option<Result<T, F>> {
    public flow transpose(self) -> Result<Option<T>, F> ![] {
        return match self {
            None => Ok(None),
            Some(result) => match result { Ok(value) => Ok(Some(value)), Err(error) => Err(error) },
        };
    }
}
