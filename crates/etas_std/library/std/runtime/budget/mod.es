module std.runtime.budget;

public type Money = { private amount: f64, private currency: string };

public flow usd(amount: f64) -> Money ![] {
    return Money { amount= amount, currency= "USD" };
}
