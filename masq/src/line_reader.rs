// Copyright (c) 2019-2021, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use crate::utils::MASQ_PROMPT;
use rustyline::Editor;
use std::io;
use std::io::{BufRead, Read};

pub struct LineReader {
    delegate: Editor<()>,
}

impl Read for LineReader {
    fn read(&mut self, _: &mut [u8]) -> Result<usize, io::Error> {
        panic!("Should never be called");
    }
}

impl BufRead for LineReader {
    fn fill_buf(&mut self) -> Result<&[u8], io::Error> {
        panic!("Should never be called");
    }

    fn consume(&mut self, _: usize) {
        panic!("Should never be called");
    }

    fn read_line(&mut self, buf: &mut String) -> Result<usize, io::Error> {
        let line = match self.delegate.readline(MASQ_PROMPT) {
            Ok(line) => line,
            Err(e) => unimplemented!("{:?}", e),
        };
        self.delegate.add_history_entry(&line);
        let len = line.len();
        buf.clear();
        buf.push_str(&line);
        Ok(len)
    }
}

impl Default for LineReader {
    fn default() -> Self {
        LineReader::new()
    }
}

impl LineReader {
    pub fn new() -> LineReader {
        LineReader {
            delegate: Editor::new(),
        }
    }
}
