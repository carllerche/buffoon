use {Serialize};
use wire_type::WireType;
use std::io;
use std::borrow::Borrow;

pub trait OutputStream {
    /// Writes a nested message with the specified field number
    fn write_message<T: Serialize>(&mut self, field: usize, msg: &T) -> io::Result<()>;

    /// Writes a field as bytes
    fn write_bytes(&mut self, field: usize, val: &[u8]) -> io::Result<()>;

    /// Write a field as a varint
    fn write_varint<T: Into<u64>>(&mut self, field: usize, val: T) -> io::Result<()>;

    fn write_packed_varint<T: Into<u64>, I>(&mut self, field: usize, vals: I) -> io::Result<()>
            where T: Into<u64>,
                  I: IntoIterator<Item=T>;

    /// Write a repeated message field
    fn write_repeated_message<T, I>(&mut self, field: usize, msgs: I) -> io::Result<()>
            where T: Serialize,
                  I: IntoIterator<Item=T> {
        for msg in msgs {
            try!(self.write_message(field, &msg));
        }

        Ok(())
    }

    /// Write each value yielded by iterator as a byte field
    fn write_repeated_bytes<'a, I>(&mut self, field: usize, vals: I) -> io::Result<()>
            where I: Iterator<Item=&'a [u8]> {

        for val in vals {
            try!(self.write_bytes(field, val));
        }

        Ok(())
    }

    fn write_string(&mut self, field: usize, val: &str) -> io::Result<()> {
        self.write_bytes(field, val.as_bytes())
    }

    fn write_opt_string<S: Borrow<str>>(&mut self, field: usize, val: Option<S>) -> io::Result<()> {
        match val {
            Some(s) => try!(self.write_string(field, s.borrow())),
            None => {}
        }

        Ok(())
    }

    fn write_repeated_string<'a, I: Iterator<Item=&'a str>>(&mut self, field: usize, vals: I) -> io::Result<()> {
        self.write_repeated_bytes(field, vals.map(|s| s.as_bytes()))
    }
}

pub trait OutputStreamImpl {
    /// Write raw bytes to the underlying stream
    fn write_raw_bytes(&mut self, bytes: &[u8]) -> io::Result<()>;

    /// Write a single byte to the underlying stream
    fn write_raw_byte(&mut self, byte: u8) -> io::Result<()> {
        let buf = [byte];
        self.write_raw_bytes(&buf)
    }

    fn write_usize(&mut self, val: usize) -> io::Result<()> {
        self.write_unsigned_varint(val as u64)
    }

    fn write_unsigned_varint(&mut self, val: u64) -> io::Result<()>;

    fn write_head(&mut self, field: usize, wire_type: WireType) -> io::Result<()> {
        // TODO: Handle overflow
        let bits = (field << 3) | (wire_type as usize);
        try!(self.write_usize(bits));
        Ok(())
    }
}
