use {Serialize, OutputStream, Varint};
use output_stream::write_head;
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

impl<'a, W: Write> OutputStream for OutputWriter<'a, W> {
    fn write<T: ?Sized + Serialize>(&mut self, field: u32, val: &T) -> io::Result<()> {
        val.serialize_nested(field, self)
    }

    fn write_nested<T: ?Sized + Serialize>(&mut self, field: u32, val: &T) -> io::Result<()> {
        if self.curr >= self.nested.len() {
            return invalid_serializer();
        }

        let size = self.nested[self.curr];
        self.curr += 1;

        if size > 0 {
            try!(write_head(self, field, WireType::LengthDelimited));
            try!(self.write_raw_varint(size));

            try!(val.serialize(self));
        };

        Ok(())
    }

    fn write_varint<T: Varint>(&mut self, field: u32, val: T) -> io::Result<()> {
        try!(write_head(self, field, WireType::Varint));
        try!(self.write_raw_varint(val));
        Ok(())
    }

    fn write_packed<T, I>(&mut self, field: u32, vals: I) -> io::Result<()>
            where T: Varint,
                  I: IntoIterator<Item=T> {
        if self.curr >= self.nested.len() {
            return invalid_serializer();
        }

        let size = self.nested[self.curr];
        self.curr += 1;

        if size > 0 {
            try!(write_head(self, field, WireType::LengthDelimited));
            try!(self.write_raw_varint(size));

            for val in vals {
                try!(self.write_raw_varint(val));
            }
        };

        Ok(())
    }

    fn write_bytes(&mut self, field: u32, val: &[u8]) -> io::Result<()> {
        try!(write_head(self, field, WireType::LengthDelimited));
        try!(self.write_raw_varint(val.len()));
        try!(self.write_raw_bytes(val));
        Ok(())
    }

    fn write_raw_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        try!(self.writer.write(bytes));
        Ok(())
    }

    fn write_raw_varint<T: Varint>(&mut self, val: T) -> io::Result<()> {
        val.write(&mut self.writer)
    }
}

fn invalid_serializer<T>() -> io::Result<T> {
    Err(io::Error::new(io::ErrorKind::Other, "invalid serializer for current message"))
}
