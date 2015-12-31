use {Serialize, WireType};
use std::io;
use std::borrow::Borrow;

pub trait OutputStream {
    /// Writes a nested message with the specified field number
    fn write_message_field<T: Serialize>(&mut self, field: usize, msg: &T) -> io::Result<()>;

    /// Writes a field as bytes
    fn write_byte_field(&mut self, field: usize, val: &[u8]) -> io::Result<()>;

    /// Write a field as a varint
    fn write_varint_field<T: NumField>(&mut self, field: usize, val: T) -> io::Result<()>;

    /// Write a repeated message field
    fn write_repeated_message_field<T: Serialize, I: IntoIterator<Item=T>>(&mut self, field: usize, msgs: I) -> io::Result<()> {
        for msg in msgs {
            try!(self.write_message_field(field, &msg));
        }

        Ok(())
    }

    fn write_repeated_byte_field<'a, I: Iterator<Item=&'a [u8]>>(&mut self, field: usize, vals: I) -> io::Result<()> {
        for val in vals {
            try!(self.write_byte_field(field, val));
        }

        Ok(())
    }

    fn write_str_field(&mut self, field: usize, val: &str) -> io::Result<()> {
        self.write_byte_field(field, val.as_bytes())
    }

    fn write_opt_str_field<S: Borrow<str>>(&mut self, field: usize, val: Option<S>) -> io::Result<()> {
        match val {
            Some(s) => try!(self.write_str_field(field, s.borrow())),
            None => {}
        }

        Ok(())
    }

    fn write_repeated_str_field<'a, I: Iterator<Item=&'a str>>(&mut self, field: usize, vals: I) -> io::Result<()> {
        self.write_repeated_byte_field(field, vals.map(|s| s.as_bytes()))
    }
}

pub trait OutputStreamImpl {
    /// Write raw bytes to the underlying stream
    fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()>;

    /// Write a single byte to the underlying stream
    fn write_byte(&mut self, byte: u8) -> io::Result<()> {
        let buf = [byte];
        self.write_bytes(&buf)
    }

    fn write_usize(&mut self, val: usize) -> io::Result<()> {
        self.write_unsigned_varint(val as u64)
    }

    fn write_unsigned_varint(&mut self, mut val: u64) -> io::Result<()> {
        loop {
            // Grab up to 7 bits of the number
            let bits = (val & 0x7f) as u8;

            // Shift the remaining bits
            val >>= 7;

            if val == 0 {
                try!(self.write_byte(bits));
                return Ok(());
            }

            try!(self.write_byte(bits | 0x80));
        }
    }

    fn write_head(&mut self, field: usize, wire_type: WireType) -> io::Result<()> {
        // TODO: Handle overflow
        let bits = (field << 3) | (wire_type as usize);
        try!(self.write_usize(bits));
        Ok(())
    }
}

pub trait NumField {
    fn write_varint_field<O: ?Sized + OutputStreamImpl>(self, field: usize, out: &mut O) -> io::Result<()>;
}

impl NumField for usize {
    fn write_varint_field<O: ?Sized + OutputStreamImpl>(self, field: usize, out: &mut O) -> io::Result<()> {
        try!(out.write_head(field, WireType::Varint));
        try!(out.write_usize(self));
        Ok(())
    }
}

impl NumField for u64 {
    fn write_varint_field<O: ?Sized + OutputStreamImpl>(self, field: usize, out: &mut O) -> io::Result<()> {
        try!(out.write_head(field, WireType::Varint));
        try!(out.write_unsigned_varint(self));
        Ok(())
    }
}

impl<F: NumField> NumField for Option<F> {
    fn write_varint_field<O: ?Sized + OutputStreamImpl>(self, field: usize, out: &mut O) -> io::Result<()> {
        match self {
            Some(v) => v.write_varint_field(field, out),
            None => Ok(())
        }
    }
}
