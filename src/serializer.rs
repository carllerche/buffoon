use {Serialize, OutputStream};
use output_stream::{OutputStreamImpl};
use output_writer::OutputWriter;
use wire_type::WireType;
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

    fn write_nested<F>(&mut self, field: usize, f: F) -> io::Result<()>
            where F: FnOnce(&mut Serializer) -> io::Result<()> {
        let position = self.nested.len();
        let prev_count = self.size;

        // Add 0 as a placeholder for the current message
        self.nested.push(0);

        try!(f(self));

        let nested_size = self.size - prev_count;

        if nested_size > 0 {
            self.nested[position] = nested_size;

            try!(self.write_head(field, WireType::LengthDelimited));
            try!(self.write_usize(nested_size));
        }

        Ok(())
    }
}

impl OutputStreamImpl for Serializer {
    fn write_raw_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        // TODO: Handle overflow
        self.size += bytes.len();
        Ok(())
    }

    fn write_unsigned_varint(&mut self, mut val: u64) -> io::Result<()> {
        // Handle a common case
        if val & (!0 << 7) == 0 {
            self.size += 1;
            return Ok(());
        }

        let mut n = 2;

        if val & (!0 << 35) != 0 {
            n += 4;
            val >>= 28;
        }

        if val & (!0 << 21) != 0 {
            n += 2;
            val >> 14;
        }

        if val & (!0 << 14) != 0 {
            n += 1;
        }

        self.size += n;

        Ok(())
    }
}

impl OutputStream for Serializer {
    fn write_message<T: Serialize>(&mut self, field: usize, msg: &T) -> io::Result<()> {
        self.write_nested(field, |me| msg.serialize(me))
    }

    fn write_bytes(&mut self, field: usize, val: &[u8]) -> io::Result<()> {
        try!(self.write_head(field, WireType::LengthDelimited));
        try!(self.write_usize(val.len()));
        try!(self.write_raw_bytes(val));
        Ok(())
    }

    fn write_varint<F: Into<u64>>(&mut self, field: usize, val: F) -> io::Result<()> {
        try!(self.write_head(field, WireType::Varint));
        try!(self.write_unsigned_varint(val.into()));
        Ok(())
    }

    fn write_packed_varint<T: Into<u64>, I>(&mut self, field: usize, vals: I) -> io::Result<()>
            where T: Into<u64>,
                  I: IntoIterator<Item=T> {

        // Compute the nested size of the packed field
        self.write_nested(field, |me| {
            for val in vals {
                try!(me.write_unsigned_varint(val.into()));
            }

            Ok(())
        })
    }
}
