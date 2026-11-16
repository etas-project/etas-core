module std.text.unicode;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// A Unicode normalization form.
public enum Normalization {
    NFC,
    NFD,
    NFKC,
    NFKD,
}

// Apply Unicode default full case folding.
@std.binding("std.text.unicode.casefold")
public flow casefold(arg0: string) -> string ![] {}

// Count extended grapheme clusters.
@std.binding("std.text.unicode.grapheme_count")
public flow grapheme_count(arg0: string) -> usize ![] {}

// Lazily traverse cluster byte positions.
@std.binding("std.text.unicode.grapheme_indices")
public flow grapheme_indices(arg0: string) -> std.collections.Iter<(usize, string)> ![] {}

// Return a checked extended-grapheme interval.
@std.binding("std.text.unicode.grapheme_slice")
public flow grapheme_slice<I ~ std.core.Index>(arg0: string, arg1: std.collections.Range<I>) -> string ![Error<std.text.TextError>] {}

// Lazily traverse extended grapheme clusters.
@std.binding("std.text.unicode.graphemes")
public flow graphemes(arg0: string) -> std.collections.Iter<string> ![] {}

// Test pinned Unicode normalization.
@std.binding("std.text.unicode.is_normalized")
public flow is_normalized(arg0: string, arg1: std.text.unicode.Normalization) -> bool ![] {}

// Normalize with pinned Unicode data.
@std.binding("std.text.unicode.normalize")
public flow normalize(arg0: string, arg1: std.text.unicode.Normalization) -> string ![] {}

// Reverse extended grapheme clusters.
@std.binding("std.text.unicode.reverse_graphemes")
public flow reverse_graphemes(arg0: string) -> string ![] {}
