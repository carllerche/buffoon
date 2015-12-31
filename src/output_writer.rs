use {Serialize, OutputStream};
use output_stream::OutputStreamImpl;
use wire_type::WireType;
use std::io::{self, Write};

pub struct OutputWriter<'a, W:'a> {
    curr: usize,
    nested: &'a [usize],
    writer: &'a mut W
}

impl<'a, W: Write> OutputWriter<'a, W> {
    pub fn new(nested: &'a [usize], writer: &'a mut W) -> OutputWriter<'a, W> {
        OutputWriter {
            curr: 0,
            nested: nested,
            writer: writer
        }
    }
}

impl<'a, W: Write> OutputStreamImpl for OutputWriter<'a, W> {
    fn write_raw_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        try!(self.writer.write(bytes));
        Ok(())
    }
}

impl<'a, W: Write> OutputStream for OutputWriter<'a, W> {
    fn write_message<T: Serialize>(&mut self, field: usize, msg: &T) -> io::Result<()> {
        if self.curr >= self.nested.len() {
            return invalid_serializer();
        }

        let size = self.nested[self.curr];
        self.curr += 1;

        if size > 0 {
            try!(self.write_head(field, WireType::LengthDelimited));
            try!(self.write_usize(size));

            try!(msg.serialize(self));
        };

        Ok(())
    }

    fn write_varint<F: Into<u64>>(&mut self, field: usize, val: F) -> io::Result<()> {
        try!(self.write_head(field, WireType::Varint));
        try!(self.write_unsigned_varint(val.into()));
        Ok(())
    }

    fn write_byte(&mut self, field: usize, val: &[u8]) -> io::Result<()> {
        try!(self.write_head(field, WireType::LengthDelimited));
        try!(self.write_usize(val.len()));
        try!(self.write_raw_bytes(val));
        Ok(())
    }
}

fn invalid_serializer<T>() -> io::Result<T> {
    Err(io::Error::new(io::ErrorKind::Other, "invalid serializer for current message"))
}
