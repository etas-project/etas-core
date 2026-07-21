use crate::HostValue;

pub trait HostValueCodec<V> {
    type Error;

    fn encode(value: &V) -> Result<HostValue, Self::Error>;
    fn decode(value: HostValue) -> Result<V, Self::Error>;
}
