use {Serialize, OutputStream, WireType};
use output_stream::OutputStreamBackend;
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

impl<'a, W: Write> OutputStreamBackend for OutputWriter<'a, W> {
    fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        try!(self.writer.write(bytes));
        Ok(())
    }
}

impl<'a, W: Write> OutputStream for OutputWriter<'a, W> {
    fn write_message_field<T: Serialize>(&mut self, field: usize, msg: &T) -> io::Result<()> {
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
}

fn invalid_serializer<T>() -> io::Result<T> {
    Err(io::Error::new(io::ErrorKind::Other, "invalid serializer for current message"))
}
