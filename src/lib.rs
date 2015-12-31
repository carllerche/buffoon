pub use input_stream::{InputStream, Field};
pub use output_stream::OutputStream;
pub use serializer::Serializer;

use std::io::{self, Read};

mod input_stream;
mod output_stream;
mod output_writer;
mod serializer;
mod wire_type;

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

#[cfg(test)]
mod test {
    use std::io;
    use super::{Serialize, OutputStream, serialize};

    #[test]
    pub fn test_writing_unit_struct() {
        struct Empty;

        impl Serialize for Empty {
            fn serialize<O: OutputStream>(&self, _: &mut O) -> io::Result<()> {
                Ok(())
            }
        }

        let bytes = serialize(&Empty).unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    pub fn test_writing_simple_message() {
        struct Simple;

        impl Serialize for Simple {
            fn serialize<O: OutputStream>(&self, out: &mut O) -> io::Result<()> {
                try!(out.write_string(1, "hello"));
                // try!(output.write_varint(2, self.config()));
                // try!(output.write_repeated_string(3, self.cmd().iter().map(|s| s.as_slice())));

                Ok(())
            }
        }

        let bytes = serialize(&Simple).unwrap();
        let expect = b"\x0A\x05hello";
        assert!(bytes == expect, "expect={:?}; actual={:?}", expect, bytes);
    }

    #[test]
    pub fn test_serializing_packed_varints() {
        struct Simple;

        impl Serialize for Simple {
            fn serialize<O: OutputStream>(&self, out: &mut O) -> io::Result<()> {
                try!(out.write_packed_varint(4, [3u64, 270, 86942].iter().map(|num| *num)));
                Ok(())
            }
        }

        let bytes = serialize(&Simple).unwrap();
        let expect = b"\x22\x06\x03\x8e\x02\x9e\xa7\x05";

        assert_eq!(bytes, expect);
    }
}
