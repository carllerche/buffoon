pub use input_stream::{InputStream, Field};
pub use output_stream::OutputStream;
pub use serializer::Serializer;

use std::io::{self, Read};

mod input_stream;
mod output_stream;
mod output_writer;
mod serializer;
mod wire_type;

pub fn load<'a, M: LoadableMessage, R: io::Read>(reader: &mut R) -> io::Result<M> {
    LoadableMessage::load(reader)
}

pub fn serializer_for<M: Message>(msg: &M) -> io::Result<Serializer> {
    let mut serializer = Serializer::new();

    // populate the message size info
    try!(msg.serialize(&mut serializer));

    Ok(serializer)
}

pub fn serialize<M: Message>(msg: &M) -> io::Result<Vec<u8>> {
    use std::iter::repeat;

    let serializer = try!(serializer_for(msg));
    let mut bytes: Vec<u8> = repeat(0).take(serializer.size()).collect();

    try!(serializer.serialize_into(msg, &mut bytes));
    Ok(bytes)
}

pub trait Message {
    fn serialize<O: OutputStream>(&self, out: &mut O) -> io::Result<()>;
}

impl<'a, M: 'a + Message> Message for &'a M {
    fn serialize<O: OutputStream>(&self, out: &mut O) -> io::Result<()> {
        (*self).serialize(out)
    }
}

pub trait LoadableMessage : Sized {
    fn load_from_stream<'a, R:'a+Read>(reader: &mut InputStream<'a, R>) -> io::Result<Self>;

    fn load<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut stream = InputStream::new(reader);
        LoadableMessage::load_from_stream(&mut stream)
    }
}

#[cfg(test)]
mod test {
    use std::io;
    use super::{Message, OutputStream, serialize};

    struct Empty;

    impl Message for Empty {
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

    impl Message for Simple {
        fn serialize<O: OutputStream>(&self, out: &mut O) -> io::Result<()> {
            try!(out.write_str_field(1, "hello"));
            // try!(output.write_varint_field(2, self.config()));
            // try!(output.write_repeated_str_field(3, self.cmd().iter().map(|s| s.as_slice())));

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
