use std::{
    io::{Error, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

use itertools::Itertools;

pub struct Econ {
    connection: TcpStream,
    buffer: Vec<u8>,
    lines: Vec<String>,
    unfinished_line: String,
    authed: bool,
}

impl Econ {
    pub fn connect<A: ToSocketAddrs>(address: A, buffer_size: usize) -> Result<Self, Error> {
        let address = address.to_socket_addrs().unwrap().next().unwrap();

        let connection = TcpStream::connect_timeout(&address, Duration::from_secs(10))?;

        Ok(Self {
            connection,
            buffer: Vec::with_capacity(buffer_size),
            lines: Vec::new(),
            unfinished_line: String::new(),
            authed: false,
        })
    }

    pub fn auth(&mut self, password: &str) -> bool {
        self.read().unwrap(); // "Enter password:"
        self.lines.clear();

        self.send_rcon_cmd(password).unwrap();

        self.read().unwrap();

        while let Some(line) = self.pop_line() {
            if line.starts_with("Authentication successful") {
                self.authed = true;
            }
        }

        self.authed
    }

    pub fn is_authed(&self) -> bool {
        self.authed
    }

    pub fn read(&mut self) -> Result<(), Error> {
        let written = self.connection.read(&mut self.buffer)?;

        if written != 0 {
            let mut lines = String::from_utf8_lossy(&self.buffer[..written])
                .replace('\0', "")
                .split("\n")
                .map(String::from)
                .collect_vec();

            if lines.last().unwrap() == "" {
                let _ = lines.pop();

                if !self.unfinished_line.is_empty() {
                    let take = self.unfinished_line.to_owned();
                    lines[0] = take + &lines[0];

                    self.unfinished_line.clear();
                }
            } else {
                self.unfinished_line = lines.pop().unwrap();
            }

            self.lines.extend(lines);
        }

        Ok(())
    }

    pub fn pop_line(&mut self) -> Option<String> {
        self.lines.pop()
    }

    pub fn send_rcon_cmd(&mut self, command: &str) -> Result<(), std::io::Error> {
        self.connection.write_all(command.as_bytes())?;
        self.connection.write_all("\n".as_bytes())?;

        self.connection.flush()?;

        Ok(())
    }
}
