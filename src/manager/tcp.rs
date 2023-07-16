use std::time::Duration;

use color_eyre::eyre::Report;
use tokio::io::AsyncReadExt;

pub struct Tcp {
    host: String,
}

impl Tcp {
    pub fn new(host: String) -> Self {
        Self { host }
    }

    pub async fn run(&self, tx: flume::Sender<String>) -> Result<(), Report> {
        let listener = tokio::net::TcpListener::bind(&self.host).await?;

        loop {
            let (socket, _) = listener.accept().await?;
            let tx = tx.clone();
            tokio::spawn(async move {
                // read everything for 30s, then timeout if it is not closed
                let read_text =
                    tokio::time::timeout(Duration::from_secs(2), Tcp::read_all(socket)).await;

                let inner = match read_text {
                    Ok(inner) => inner,
                    Err(err) => {
                        // timeout
                        eprintln!("Timedout {:?}", err);
                        return;
                    }
                };

                let text = match inner {
                    Ok(text) => text,
                    Err(err) => {
                        // read_all failed
                        eprintln!("Readall failed {:?}", err);
                        return;
                    }
                };

                let text = String::from_utf8(text).unwrap();
                tx.send(text).unwrap();
            });
        }
    }

    async fn read_all(mut socket: tokio::net::TcpStream) -> Result<Vec<u8>, std::io::Error> {
        let mut buf = [0; 1024];
        let mut read_text = Vec::new();
        loop {
            match socket.read(&mut buf).await {
                Ok(n) => {
                    if n == 0 {
                        break;
                    }
                    read_text.extend(&buf[..n]);
                }
                Err(e) => {
                    return Result::Err(e);
                }
            }
        }
        Ok(read_text)
    }
}
