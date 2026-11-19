module std.text.unicode;

public import std.text.unicode.Normalization.{NFC, NFD, NFKC, NFKD};
public enum Normalization {
    NFC,
    NFD,
    NFKC,
    NFKD,
}
