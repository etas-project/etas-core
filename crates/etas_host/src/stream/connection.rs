use std::{collections::VecDeque, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
    sync::Mutex,
};
use tokio_rustls::client::TlsStream;

#[derive(Clone)]
pub(crate) enum ManagedStream {
    File {
        file: Arc<Mutex<Option<tokio::fs::File>>>,
        mode: crate::FileMode,
        read_buffer: Arc<Mutex<VecDeque<u8>>>,
    },
    Tcp {
        stream: Arc<TcpStream>,
        read_buffer: Arc<Mutex<VecDeque<u8>>>,
    },
    Tls {
        reader: Arc<Mutex<ReadHalf<Box<TlsStream<TcpStream>>>>>,
        writer: Arc<Mutex<WriteHalf<Box<TlsStream<TcpStream>>>>>,
        read_buffer: Arc<Mutex<VecDeque<u8>>>,
    },
}

impl ManagedStream {
    pub(crate) fn file(file: std::fs::File, mode: crate::FileMode) -> Self {
        Self::File {
            file: Arc::new(Mutex::new(Some(tokio::fs::File::from_std(file)))),
            mode,
            read_buffer: Default::default(),
        }
    }

    pub(crate) fn validate_access(&self, write: bool) -> Result<(), crate::HostError> {
        if let Self::File { mode, .. } = self {
            if if write {
                !mode.writable()
            } else {
                !mode.readable()
            } {
                return Err(crate::HostError::new(
                    crate::HostErrorCode::Unsupported,
                    "file mode does not support this stream operation",
                )
                .with_detail("stream_failure", "unsupported_operation"));
            }
        }
        Ok(())
    }

    pub(crate) async fn finish_file(self) {
        if let Self::File { file, .. } = self {
            if let Some(file) = file.lock().await.take() {
                // into_std waits for Tokio's already-dispatched blocking file work.
                drop(file.into_std().await);
            }
        }
    }

    pub(crate) async fn sync_file(&self) -> std::io::Result<()> {
        let Self::File { file, .. } = self else {
            return Err(std::io::ErrorKind::InvalidInput.into());
        };
        let mut guard = file.lock().await;
        let file = guard
            .as_mut()
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotConnected))?;
        file.sync_all().await
    }

    pub(crate) fn tcp(stream: TcpStream) -> Self {
        Self::Tcp {
            stream: Arc::new(stream),
            read_buffer: Default::default(),
        }
    }

    pub(crate) fn tls(stream: TlsStream<TcpStream>) -> Self {
        let (reader, writer) = tokio::io::split(Box::new(stream));
        Self::Tls {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
            read_buffer: Default::default(),
        }
    }

    fn read_buffer(&self) -> &Mutex<VecDeque<u8>> {
        match self {
            Self::Tcp { read_buffer, .. }
            | Self::Tls { read_buffer, .. }
            | Self::File { read_buffer, .. } => read_buffer,
        }
    }

    pub(crate) async fn read(&self, output: &mut [u8]) -> std::io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        {
            let mut buffered = self.read_buffer().lock().await;
            let count = output.len().min(buffered.len());
            if count != 0 {
                for byte in &mut output[..count] {
                    *byte = buffered.pop_front().unwrap();
                }
                return Ok(count);
            }
        }
        match self {
            Self::Tcp { stream, .. } => loop {
                stream.readable().await?;
                match stream.try_read(output) {
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
                    result => return result,
                }
            },
            Self::Tls { reader, .. } => reader.lock().await.read(output).await,
            Self::File { file, .. } => {
                let mut guard = file.lock().await;
                guard
                    .as_mut()
                    .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotConnected))?
                    .read(output)
                    .await
            }
        }
    }

    pub(crate) async fn prepend_read_buffer(&self, bytes: Vec<u8>) {
        let mut buffered = self.read_buffer().lock().await;
        for byte in bytes.into_iter().rev() {
            buffered.push_front(byte);
        }
    }

    pub(crate) fn is_plain_tcp_without_buffered_data(&self) -> bool {
        matches!(self, Self::Tcp { .. })
            && self
                .read_buffer()
                .try_lock()
                .is_ok_and(|buffer| buffer.is_empty())
    }

    pub(crate) async fn write(&self, body: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp { stream, .. } => loop {
                stream.writable().await?;
                match stream.try_write(body) {
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
                    result => return result,
                }
            },
            Self::Tls { writer, .. } => writer.lock().await.write(body).await,
            Self::File {
                file, read_buffer, ..
            } => {
                let mut guard = file.lock().await;
                let file = guard
                    .as_mut()
                    .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotConnected))?;
                let mut buffered = read_buffer.lock().await;
                if !buffered.is_empty() {
                    let unread = i64::try_from(buffered.len())
                        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
                    file.seek(std::io::SeekFrom::Current(-unread)).await?;
                    buffered.clear();
                }
                let written = file.write(body).await?;
                file.flush().await?;
                Ok(written)
            }
        }
    }

    pub(crate) async fn flush(&self) -> std::io::Result<()> {
        match self {
            Self::Tcp { .. } => Ok(()),
            Self::Tls { writer, .. } => writer.lock().await.flush().await,
            Self::File { file, .. } => {
                let mut guard = file.lock().await;
                guard
                    .as_mut()
                    .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotConnected))?
                    .flush()
                    .await
            }
        }
    }
}
