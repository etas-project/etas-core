etas_core::id_type!(TraceSpanId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraceId(pub u128);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceContext {
    pub trace_id: TraceId,
    pub parent_trace: Option<TraceId>,
    pub parent_span: Option<TraceSpanId>,
}

impl TraceContext {
    pub fn root(trace_id: TraceId) -> Self {
        Self {
            trace_id,
            parent_trace: None,
            parent_span: None,
        }
    }

    pub fn resumed(trace_id: TraceId, parent_trace: TraceId) -> Self {
        Self {
            trace_id,
            parent_trace: Some(parent_trace),
            parent_span: None,
        }
    }
}

impl TraceId {
    pub fn generate() -> Result<Self, crate::HostError> {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).map_err(|error| {
            crate::HostError::new(
                crate::HostErrorCode::ProviderUnavailable,
                format!("failed to generate a resumed trace identity: {error}"),
            )
        })?;
        Ok(Self(u128::from_le_bytes(bytes)))
    }

    pub fn to_hex(self) -> String {
        format!("{:032x}", self.0)
    }

    pub fn from_hex(value: &str) -> Result<Self, &'static str> {
        if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("trace identity must be exactly 32 hexadecimal characters");
        }
        u128::from_str_radix(value, 16)
            .map(Self)
            .map_err(|_| "trace identity contains an invalid hexadecimal value")
    }
}

impl std::fmt::Display for TraceId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

impl etas_core::serde::Serialize for TraceId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: etas_core::serde::Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> etas_core::serde::Deserialize<'de> for TraceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: etas_core::serde::Deserializer<'de>,
    {
        let value = <String as etas_core::serde::Deserialize>::deserialize(deserializer)?;
        Self::from_hex(&value).map_err(etas_core::serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::TraceId;

    #[test]
    fn trace_identity_uses_canonical_128_bit_hex() {
        let trace = TraceId(0x1234);
        assert_eq!(trace.to_hex(), "00000000000000000000000000001234");
        assert_eq!(TraceId::from_hex(&trace.to_hex()), Ok(trace));
        assert!(TraceId::from_hex("1234").is_err());
    }
}
