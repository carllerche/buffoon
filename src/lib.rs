pub use input_stream::{InputStream, Field};
pub use output_stream::OutputStream;
pub use serializer::Serializer;

use std::io::{self, Read};

mod input_stream;
mod output_stream;
mod output_writer;
mod serializer;

/// Deserialize an encoded Protocol Buffers message.
pub fn deserialize<T, R, I>(input: I) -> io::Result<T>
        where T: Deserialize,
              R: io::Read,
              I: Into<InputStream<R>> {
    let mut input = input.into();
    T::deserialize(&mut input)
}

pub fn serializer_for<T: Serialize>(msg: &T) -> io::Result<Serializer> {
    let mut serializer = Serializer::new();

    // populate the message size info
    try!(msg.serialize(&mut serializer));

    Ok(serializer)
}

pub fn serialize<T: Serialize>(msg: &T) -> io::Result<Vec<u8>> {
    let serializer = try!(serializer_for(msg));
    let mut bytes = vec![0u8; serializer.size()];

    try!(serializer.serialize_into(msg, &mut bytes));
    Ok(bytes)
}

pub trait Serialize {
    fn serialize<O>(&self, out: &mut O) -> io::Result<()> where O: OutputStream;
}

impl<'a, T: 'a + Serialize> Serialize for &'a T {
    fn serialize<O: OutputStream>(&self, out: &mut O) -> io::Result<()> {
        (*self).serialize(out)
    }
}

pub trait Deserialize {
    fn deserialize<R: Read>(reader: &mut InputStream<R>) -> io::Result<Self>
            where Self: Sized;
}

#[derive(Debug)]
enum WireType {
    Varint = 0,
    SixtyFourBit = 1,
    LengthDelimited = 2,
    StartGroup = 3,
    EndGroup = 4,
    ThirtyTwoBit = 5
}

impl WireType {
    pub fn from_usize(val: usize) -> Option<WireType> {
        use self::WireType::*;

        Some(match val {
            0 => Varint,
            1 => SixtyFourBit,
            2 => LengthDelimited,
            3 => StartGroup,
            4 => EndGroup,
            5 => ThirtyTwoBit,
            _ => return None
        })
    }
}

#[cfg(test)]
mod test {
    use std::io;
    use super::{Serialize, OutputStream, serialize};

    struct Empty;

    impl Serialize for Empty {
        fn serialize<O: OutputStream>(&self, _: &mut O) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    pub fn test_writing_unit_struct() {
        let bytes = serialize(&Empty).unwrap();
        assert!(bytes.is_empty());
    }

    struct Simple;

    impl Serialize for Simple {
        fn serialize<O: OutputStream>(&self, out: &mut O) -> io::Result<()> {
            try!(out.write_str(1, "hello"));
            // try!(output.write_varint(2, self.config()));
            // try!(output.write_repeated_str(3, self.cmd().iter().map(|s| s.as_slice())));

            Ok(())
        }
    }

    #[test]
    pub fn test_writing_simple_message() {
        let bytes = serialize(&Simple).unwrap();
        let expect = b"\x0A\x05hello";
        assert!(bytes == expect, "expect={:?}; actual={:?}", expect, bytes);
    }
}
