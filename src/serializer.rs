use {Serialize, OutputStream, WireType};
use output_stream::{OutputStreamImpl, NumField};
use output_writer::OutputWriter;
use std::io;

pub struct Serializer {
    size: usize,
    nested: Vec<usize>
}

impl Serializer {
    pub fn new() -> Serializer {
        Serializer {
            size: 0,
            nested: Vec::new()
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn serialize<T: Serialize, W: io::Write>(&self, msg: &T, writer: &mut W) -> io::Result<()> {
        let mut out = OutputWriter::new(&self.nested, writer);

        try!(msg.serialize(&mut out));

        Ok(())
    }

    pub fn serialize_into<T: Serialize>(&self, msg: &T, dst: &mut [u8]) -> io::Result<()> {
        if self.size > dst.len() {
            return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "destination buffer not large enough to contain serialized message"));
        }

        self.serialize(msg, &mut io::BufWriter::new(dst))
    }
}

impl OutputStreamImpl for Serializer {
    fn write_raw_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        // TODO: Handle overflow
        self.size += bytes.len();
        Ok(())
    }
}

impl OutputStream for Serializer {
    fn write_message<T: Serialize>(&mut self, field: usize, msg: &T) -> io::Result<()> {
        let position = self.nested.len();
        let prev_count = self.size;

        // Add 0 as a placeholder for the current message
        self.nested.push(0);

        try!(msg.serialize(self));

        let nested_size = self.size - prev_count;

        if nested_size > 0 {
            self.nested[position] = nested_size;

            try!(self.write_head(field, WireType::LengthDelimited));
            try!(self.write_usize(nested_size));
        }

        Ok(())
    }

    fn write_byte(&mut self, field: usize, val: &[u8]) -> io::Result<()> {
        try!(self.write_head(field, WireType::LengthDelimited));
        try!(self.write_usize(val.len()));
        try!(self.write_raw_bytes(val));
        Ok(())
    }

    fn write_varint<F: NumField>(&mut self, field: usize, val: F) -> io::Result<()> {
        val.write_varint(field, self)
    }
}
