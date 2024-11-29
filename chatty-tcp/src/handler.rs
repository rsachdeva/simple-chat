use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;

pub struct ChatHandler {
    pub writer_half: OwnedWriteHalf,
    pub reader_half: OwnedReadHalf,
}

impl ChatHandler {
    pub fn new(stream: TcpStream) -> Self {
        let (read, write) = stream.into_split();
        Self {
            writer_half: write,
            reader_half: read,
        }
    }
}
