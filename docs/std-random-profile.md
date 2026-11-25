# Seeded random profile

`std.random.Rng` uses SplitMix64, profile version 1. Its sole private field is
the current 64-bit state. `new(seed)` stores the seed unchanged. Every
`next_u64` advances the state by `0x9e3779b97f4a7c15` modulo 2^64, then applies
the SplitMix64 finalizer to that successor state. This algorithm is implemented
in Etas and does not use host entropy. It is not a cryptographic generator.

For seed zero, the first three output words are:

| Draw | Hexadecimal output |
|---|---|
| 1 | `e220a8397b1dcdaf` |
| 2 | `6e789e6aa1b965f4` |
| 3 | `06c45d188009454f` |

`next_f64` consumes one word, takes its high 53 bits, and returns that integer
divided by 2^53 exactly. Integer sampling draws the target width uniformly,
rejects values below `2^width mod interval_length`, and then reduces modulo
the interval length. A 128-bit draw concatenates two consecutive words, first
word high. Signed intervals use the monotonic sign-bit-biased encoding.

`choose` samples an index and consumes no draw on an empty array. `shuffle`
uses descending Fisher–Yates, choosing from `[0, remaining)` at each step;
empty and singleton arrays consume no draws. All APIs return the successor
generator explicitly, so a copied generator repeats the same future stream.

The profile is part of numerical/replay compatibility. Any future algorithm,
word order, conversion or sampling change must use a new compatibility profile;
it cannot silently replace this sequence with a host generator.

## Platform entropy

`std.random.secure_bytes(count)` requests the source-declared
`std.effects.Random.entropy` action. Zero returns empty bytes without making a
request. The default implementation uses the operating system cryptographic
random source on a managed blocking worker. It never falls back to `Rng` or a
clock. The caller must hold the current `Random.entropy` action grant.

The request inherits the run budget and result byte limit. A missing platform
source raises `Error<RandomError>` with `Unavailable`; other host failures use
`Host(message)`. A source Handler takes precedence over the default implementation.
The default's nondeterminism and runtime requirement are guarded by this action,
so intercepting the action removes that default behavior from the caller summary.

Host trace metadata records the requested byte count, not returned entropy.
Successful bytes and typed failures are retained in the replay ledger. CLI
checkpoint persistence uses its existing sealed checkpoint format. Replaying a
retained occurrence returns its bytes or raises its original failure without
requesting new entropy; unavailable checkpoint keys remain a replay error.
Returned bytes remain ordinary data and are not automatically secret-wrapped.
