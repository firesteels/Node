// Copyright (c) 2019-2021, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use crate::line_reader::LineReader;
use std::io;
use std::io::{BufRead, BufReader};

pub const MASQ_PROMPT: &str = "masq> ";

pub trait BufReadFactory {
    fn make_interactive(&self) -> Box<dyn BufRead>;
    fn make_non_interactive(&self) -> Box<dyn BufRead>;
}

pub struct BufReadFactoryReal {}

impl BufReadFactory for BufReadFactoryReal {
    fn make_interactive(&self) -> Box<dyn BufRead> {
        Box::new(LineReader::new())
    }

    fn make_non_interactive(&self) -> Box<dyn BufRead> {
        Box::new(BufReader::new(io::stdin()))
    }
}

impl Default for BufReadFactoryReal {
    fn default() -> Self {
        BufReadFactoryReal::new()
    }
}

impl BufReadFactoryReal {
    pub fn new() -> BufReadFactoryReal {
        BufReadFactoryReal {}
    }
}
