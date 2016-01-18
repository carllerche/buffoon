use {Serialize, OutputStream, Varint};
use output_stream::write_head;
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

    fn write_nested<F>(&mut self, field: u32, f: F) -> io::Result<()>
            where F: FnOnce(&mut Serializer) -> io::Result<()> {
        let position = self.nested.len();
        let prev_count = self.size;

        // Add 0 as a placeholder for the current message
        self.nested.push(0);

        trace!("----> counting nested");

        try!(f(self));

        let nested_size = self.size - prev_count;

        trace!("----> serialized nested; size={}; pos={}", nested_size, position);

        if nested_size > 0 {
            self.nested[position] = nested_size;

            try!(write_head(self, field, WireType::LengthDelimited));
            try!(self.write_raw_varint(nested_size));
        } else {
            self.nested.truncate(position + 1);
        }

        Ok(())
    }
}

#[doc(hidden)]
impl OutputStream for Serializer {
    fn write<T: ?Sized + Serialize>(&mut self, field: u32, val: &T) -> io::Result<()> {
        val.serialize_nested(field, self)
    }

    fn write_nested<T: ?Sized + Serialize>(&mut self, field: u32, val: &T) -> io::Result<()> {
        Serializer::write_nested(self, field, |me| val.serialize(me))
    }

    fn write_varint<T: Varint>(&mut self, field: u32, val: T) -> io::Result<()> {
        try!(write_head(self, field, WireType::Varint));
        try!(self.write_raw_varint(val));
        Ok(())
    }

    fn write_bytes(&mut self, field: u32, val: &[u8]) -> io::Result<()> {
        try!(write_head(self, field, WireType::LengthDelimited));
        val.serialize(self)
    }

    fn write_packed<T, I>(&mut self, field: u32, vals: I) -> io::Result<()>
            where T: Varint,
                  I: IntoIterator<Item=T> {

        // Compute the nested size of the packed field
        self.write_nested(field, |me| {
            for val in vals {
                try!(me.write_raw_varint(val));
            }

            Ok(())
        })
    }

    fn write_raw_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        // TODO: Handle overflow
        self.size += bytes.len();
        Ok(())
    }

    fn write_raw_varint<T: Varint>(&mut self, val: T) -> io::Result<()> {
        let len = val.wire_len();
        trace!("Serializer::write_raw_varint; len={:?}", len);
        self.size += len;
        Ok(())
    }
}
