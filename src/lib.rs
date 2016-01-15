pub use input_stream::{InputStream, Field};
pub use output_stream::OutputStream;
pub use serializer::Serializer;
pub use types::Varint;

use std::io::{self, Read};

mod input_stream;
mod output_stream;
mod output_writer;
mod serializer;
mod types;
mod wire_type;

/// Deserialize an encoded Protocol Buffers message.
pub fn deserialize<T, R>(input: R) -> io::Result<T>
        where T: Deserialize,
              R: io::Read {
    T::deserialize(&mut input_stream::from(input))
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

/// A trait for values which can be serialized
pub trait Serialize {
    /// Serialize the value to the given output stream.
    fn serialize<O>(&self, out: &mut O) -> io::Result<()> where O: OutputStream;

    /*
     *
     * ===== Used for internal implementations =====
     *
     */

    #[doc(hidden)]
    fn serialize_nested<O: OutputStream>(&self, field: u32, out: &mut O) -> io::Result<()> {
        out.write_nested(field, self)
    }
}

/// A trait for values which can be deserialized
pub trait Deserialize : Sized {
    /// Deserialize the value
    fn deserialize<R: Read>(input: &mut InputStream<R>) -> io::Result<Self>;

    #[doc(hidden)]
    fn deserialize_nested<R: Read>(field: &mut Field<R>) -> io::Result<Self> {
        field.read_nested()
    }
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
                try!(out.write(1, "hello"));
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
                try!(out.write_packed(4, [3u64, 270, 86942].iter().map(|num| *num)));
                Ok(())
            }
        }

        let bytes = serialize(&Simple).unwrap();
        let expect = b"\x22\x06\x03\x8e\x02\x9e\xa7\x05";

        assert_eq!(bytes, expect);
    }
}
