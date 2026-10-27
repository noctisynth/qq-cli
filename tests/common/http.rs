//! Original bounded loopback fixture, never a live application endpoint.
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub struct Server {
    start: Sender<()>,
    task: JoinHandle<()>,
}

impl Server {
    /// Begin the bounded request loop only after the managed fixture is ready.
    pub fn start(self) -> JoinHandle<()> {
        self.start.send(()).unwrap();
        self.task
    }
}

pub fn serve(
    count: usize,
    mut handler: impl FnMut(&str, &Value) -> Vec<u8> + Send + 'static,
) -> (String, Server) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}/", listener.local_addr().unwrap());
    let (start, ready) = mpsc::channel();
    let task = std::thread::spawn(move || {
        // Dropping an unstarted fixture cancels it; setup is not request time.
        if ready.recv().is_err() {
            return;
        }
        for accepted in 0..count {
            let until = Instant::now() + Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < until =>
                    {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(error) => {
                        panic!("Synthetic listener after {accepted}/{count} requests: {error}")
                    }
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
                assert!(request.len() < 65536);
            }
            let headers = String::from_utf8(request).unwrap();
            assert!(
                headers
                    .to_ascii_lowercase()
                    .contains("authorization: bearer synthetic-cli-token")
            );
            let length = headers
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            assert!(length <= 1024 * 1024);
            let mut bytes = vec![0; length];
            stream.read_exact(&mut bytes).unwrap();
            let payload: Value = serde_json::from_slice(&bytes).unwrap();
            let action = headers
                .lines()
                .next()
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap()
                .trim_start_matches('/');
            let response = handler(action, &payload);
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",response.len()).unwrap();
            stream.write_all(&response).unwrap();
        }
    });
    (endpoint, Server { start, task })
}
