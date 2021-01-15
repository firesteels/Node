// Copyright (c) 2019-2021, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use crate::protocols::utils::{Direction, MarshalError, UnrecognizedData, ParseError, OpcodeData, u16_at, u16_into, Packet};
use crate::protocols::pmp::get_packet::GetOpcodeData;
use std::convert::TryFrom;

#[derive (Clone, PartialEq, Debug)]
pub enum Opcode {
    Get,
    MapUdp,
    MapTcp,
    Other(u8),
}

impl From<u8> for Opcode {
    fn from(input: u8) -> Self {
        match input & 0x7F {
            0 => Opcode::Get,
            1 => Opcode::MapUdp,
            2 => Opcode::MapTcp,
            x => Opcode::Other (x)
        }
    }
}

impl Opcode {
    pub fn code (&self) -> u8 {
        match self {
            Opcode::Get => 0,
            Opcode::MapUdp => 1,
            Opcode::MapTcp => 2,
            Opcode::Other(code) => code & 0x7F,
        }
    }

    pub fn parse_data (&self, direction: Direction, buf: &[u8]) -> Result<Box<dyn PmpOpcodeData>, ParseError> {
        match self {
            Opcode::Get => Ok(Box::new (GetOpcodeData::new(direction, buf)?)),
            Opcode::MapUdp => unimplemented!(),
            Opcode::MapTcp => unimplemented!(),
            Opcode::Other(_) => Ok(Box::new (UnrecognizedData::new())),
        }
    }
}

pub trait PmpOpcodeData: OpcodeData {}

impl PmpOpcodeData for UnrecognizedData {}

pub struct PmpPacket {
    pub version: u8,
    pub direction: Direction,
    pub opcode: Opcode,
    pub result_code_opt: Option<u16>,
    pub opcode_data: Box<dyn PmpOpcodeData>,
}

impl Default for PmpPacket {
    fn default() -> Self {
        Self {
            version: 0,
            direction: Direction::Request,
            opcode: Opcode::Other(127),
            result_code_opt: None,
            opcode_data: Box::new(UnrecognizedData::new())
        }
    }
}

impl TryFrom<&[u8]> for PmpPacket {
    type Error = ParseError;

    fn try_from(buffer: &[u8]) -> Result<Self, Self::Error> {
        let mut result = PmpPacket {
            version: 0,
            direction: Direction::Request,
            opcode: Opcode::Other (0),
            result_code_opt: None,
            opcode_data: Box::new (UnrecognizedData::new()),
        };
        if buffer.len() < 2 {
            return Err (ParseError::ShortBuffer)
        }
        result.version = buffer[0];
        result.direction = Direction::from (buffer[1]);
        result.opcode = Opcode::from (buffer[1]);
        let position = match result.direction {
            Direction::Request => {
                result.result_code_opt = None;
                2
            },
            Direction::Response => {
                if buffer.len() < 4 {
                    return Err (ParseError::ShortBuffer)
                }
                result.result_code_opt = Some (u16_at (buffer, 2));
                4
            }
        };
        result.opcode_data = result.opcode.parse_data(result.direction, &buffer[position..])?;
        Ok(result)
    }
}

impl Packet for PmpPacket {
    fn marshal(&self, buffer: &mut [u8]) -> Result<usize, MarshalError> {
        let header_len = match self.direction {
            Direction::Request => 2,
            Direction::Response => 4,
        };
        let required_len = header_len + self.opcode_data.len(self.direction);
        if buffer.len() < required_len {
            return Err (MarshalError::ShortBuffer)
        }
        buffer[0] = self.version;
        buffer[1] = self.direction.code() | self.opcode.code();
        let mut position = 2;
        if self.direction == Direction::Response {
            u16_into(buffer, 2, self.result_code_opt.unwrap_or (0x0000));
            position = 4;
        }
        self.opcode_data.marshal (self.direction, &mut buffer[position..])?;
        Ok (required_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::pmp::get_packet::GetOpcodeData;
    use std::net::Ipv4Addr;

    #[test]
    fn from_works_for_unknown_request() {
        let buffer: &[u8] = &[
            0x12, 0x55, // version, direction, opcode
        ];

        let subject = PmpPacket::try_from (buffer).unwrap();

        assert_eq! (subject.version, 0x12);
        assert_eq! (subject.direction, Direction::Request);
        assert_eq! (subject.opcode, Opcode::Other(0x55));
        assert_eq! (subject.result_code_opt, None);
        assert_eq! (subject.opcode_data.as_any().downcast_ref::<UnrecognizedData>().unwrap(), &UnrecognizedData::new());
    }

    #[test]
    fn from_works_for_get_request() {
        let buffer: &[u8] = &[
            0x00, 0x00, // version, direction, opcode
        ];

        let subject = PmpPacket::try_from (buffer).unwrap();

        assert_eq! (subject.version, 0x00);
        assert_eq! (subject.direction, Direction::Request);
        assert_eq! (subject.opcode, Opcode::Get);
        assert_eq! (subject.result_code_opt, None);
        let opcode_data = subject.opcode_data.as_any().downcast_ref::<GetOpcodeData>().unwrap();
        assert_eq! (opcode_data, &GetOpcodeData {
            epoch_opt: None,
            external_ip_address_opt: None,
        })
    }

    #[test]
    fn from_works_for_unknown_response() {
        let buffer: &[u8] = &[
            0x13, 0xD5, 0xA5, 0x5A, // version, direction, opcode, result code
        ];

        let subject = PmpPacket::try_from (buffer).unwrap();

        assert_eq! (subject.version, 0x13);
        assert_eq! (subject.direction, Direction::Response);
        assert_eq! (subject.opcode, Opcode::Other(0x55));
        assert_eq! (subject.result_code_opt, Some (0xA55A));
        assert_eq! (subject.opcode_data.as_any().downcast_ref::<UnrecognizedData>().unwrap(), &UnrecognizedData::new());
    }

    #[test]
    fn short_buffer_causes_problems_for_parsing_request() {
        let buffer: &[u8] = &[0x00u8];

        let result = PmpPacket::try_from (buffer).err();

        assert_eq! (result, Some (ParseError::ShortBuffer));
    }

    #[test]
    fn short_buffer_causes_problems_for_parsing_response() {
        let buffer: &[u8] = &[0x00u8, 0x80, 0x00];

        let result = PmpPacket::try_from (buffer).err();

        assert_eq! (result, Some (ParseError::ShortBuffer));
    }

    #[test]
    fn marshal_works_for_unknown_request() {
        let mut buffer = [0u8; 2];
        let subject = PmpPacket {
            version: 0x12,
            direction: Direction::Request,
            opcode: Opcode::Other(0x55),
            result_code_opt: None,
            opcode_data: Box::new (UnrecognizedData::new()),
        };

        let result = subject.marshal(&mut buffer).unwrap();

        assert_eq! (result, 2);
        let expected_buffer: [u8; 2] = [
            0x12, 0x55, // version, direction, opcode
        ];
        assert_eq! (buffer, expected_buffer);
    }

    #[test]
    fn marshal_works_for_get_request() {
        let mut buffer = [0u8; 2];
        let subject = PmpPacket {
            version: 0x12,
            direction: Direction::Request,
            opcode: Opcode::Get,
            result_code_opt: None,
            opcode_data: Box::new (GetOpcodeData {
                epoch_opt: None,
                external_ip_address_opt: None
            }),
        };

        let result = subject.marshal(&mut buffer).unwrap();

        assert_eq! (result, 2);
        let expected_buffer: [u8; 2] = [
            0x12, 0x00, // version, direction, opcode
        ];
        assert_eq! (buffer, expected_buffer);
    }

    #[test]
    fn marshal_works_for_unknown_response() {
        let mut buffer = [0u8; 4];
        let subject = PmpPacket {
            version: 0x13,
            direction: Direction::Response,
            opcode: Opcode::Other(0x55),
            result_code_opt: Some(0xBBAA),
            opcode_data: Box::new (UnrecognizedData::new()),
        };

        let result = subject.marshal(&mut buffer).unwrap();

        assert_eq! (result, 4);
        let expected_buffer: [u8; 4] = [
            0x13, 0xD5, 0xBB, 0xAA, // version, direction, opcode, result code
        ];
        assert_eq! (buffer, expected_buffer);
    }

    #[test]
    fn short_buffer_causes_problems_for_marshalling () {
        let mut buffer = [0x00u8; 11];
        let subject = PmpPacket {
            version: 0,
            direction: Direction::Response,
            opcode: Opcode::Get,
            result_code_opt: Some (0xABBA),
            opcode_data: Box::new (GetOpcodeData {
                epoch_opt: Some (1234),
                external_ip_address_opt: Some (Ipv4Addr::new (4,3,2,1))
            })
        };

        let result = subject.marshal (&mut buffer);

        assert_eq! (result, Err (MarshalError::ShortBuffer));
    }

    #[test]
    fn opcode_code_works () {
        assert_eq! (Opcode::Get.code(), 0);
        assert_eq! (Opcode::MapUdp.code(), 1);
        assert_eq! (Opcode::MapTcp.code(), 2);
        assert_eq! (Opcode::Other(42).code(), 42);
        assert_eq! (Opcode::Other(255).code(), 127);
    }

    #[test]
    fn opcode_from_works () {
        assert_eq! (Opcode::from (0x00), Opcode::Get);
        assert_eq! (Opcode::from (0x01), Opcode::MapUdp);
        assert_eq! (Opcode::from (0x02), Opcode::MapTcp);
        assert_eq! (Opcode::from (0x03), Opcode::Other(3));
        assert_eq! (Opcode::from (0x7F), Opcode::Other(127));
        assert_eq! (Opcode::from (0x80), Opcode::Get);
        assert_eq! (Opcode::from (0x81), Opcode::MapUdp);
        assert_eq! (Opcode::from (0x82), Opcode::MapTcp);
        assert_eq! (Opcode::from (0x83), Opcode::Other(3));
        assert_eq! (Opcode::from (0xFF), Opcode::Other(127));
    }
}