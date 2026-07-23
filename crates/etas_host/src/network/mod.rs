mod local;
pub mod tcp;

pub use local::LocalTcpClient;
pub use tcp::{
    TcpClient, TcpConnectOperation, TcpConnectRequest, TcpConnectResponse, TcpEndpoint,
    TcpStreamRef, UnavailableTcpClient,
};
