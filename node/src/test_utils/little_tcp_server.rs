// Copyright (c) 2017-2019, Substratum LLC (https://substratum.net) and/or its affiliates. All rights reserved.

use std::io::Read;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::str::FromStr;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

pub struct LittleTcpServer {
    port: u16,
    tx: Sender<()>,
    count_rx: Receiver<()>,
}

impl Drop for LittleTcpServer {
    fn drop(&mut self) {
        self.tx.send(()).unwrap();
    }
}

impl LittleTcpServer {
    pub fn start() -> LittleTcpServer {
        let listener =
            TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel();
        let (count_tx, count_rx) = mpsc::channel();
        thread::spawn(move || {
            let mut buf = [0u8; 1024];
            loop {
                if rx.try_recv().is_ok() {
                    return;
                }
                match listener.accept() {
                    Err(_) => continue,
                    Ok((mut stream, _)) => {
                        count_tx.send(()).is_ok();
                        stream
                            .set_read_timeout(Some(Duration::from_millis(100)))
                            .unwrap();
                        loop {
                            if rx.try_recv().is_ok() {
                                return;
                            }
                            match stream.read(&mut buf) {
                                Err(_) => break,
                                Ok(len) if len == 0 => break,
                                Ok(_) => stream.write(&buf).unwrap(),
                            };
                        }
                    }
                }
            }
        });
        LittleTcpServer { port, tx, count_rx }
    }

    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(IpAddr::from_str("127.0.0.1").unwrap(), self.port)
    }

    pub fn count_connections(&self, wait_for: Duration) -> u16 {
        thread::sleep(wait_for);
        let mut count = 0;
        while self.count_rx.try_recv().is_ok() {
            count += 1;
        }
        count
    }
}