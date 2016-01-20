use {Deserialize, Field, InputStream, OutputStream, Serialize};
use std::io;

impl<'a, T: 'a + Serialize> Serialize for &'a T {
    fn serialize<O: OutputStream>(&self, out: &mut O) -> io::Result<()> {
        (*self).serialize(out)
    }

    fn serialize_nested<O: OutputStream>(&self, field: u32, out: &mut O) -> io::Result<()> {
        (*self).serialize_nested(field, out)
    }
}

/*
 *
 * ===== Vec & String =====
 *
 */

impl Serialize for [u8] {
    fn serialize<O: OutputStream>(&self, _: &mut O) -> io::Result<()> {
        unimplemented!();
    }

    fn serialize_nested<O: OutputStream>(&self, field: u32, out: &mut O) -> io::Result<()> {
        out.write_bytes(field, self)
    }
}

impl Serialize for Vec<u8> {
    fn serialize<O: OutputStream>(&self, _: &mut O) -> io::Result<()> {
        unimplemented!();
    }

    fn serialize_nested<O: OutputStream>(&self, field: u32, out: &mut O) -> io::Result<()> {
        (&**self).serialize_nested(field, out)
    }
}

impl Deserialize for Vec<u8> {
    fn deserialize<R: io::Read>(_: &mut InputStream<R>) -> io::Result<Self> {
        unimplemented!();
    }

    fn deserialize_nested<R: io::Read>(field: &mut Field<R>) -> io::Result<Vec<u8>> {
        field.read_bytes()
    }
}

impl Serialize for str {
    fn serialize<O: OutputStream>(&self, _: &mut O) -> io::Result<()> {
        unimplemented!();
    }

    fn serialize_nested<O: OutputStream>(&self, field: u32, out: &mut O) -> io::Result<()> {
        out.write_bytes(field, self.as_bytes())
    }
}

impl Serialize for String {
    fn serialize<O: OutputStream>(&self, _: &mut O) -> io::Result<()> {
        unimplemented!();
    }

    fn serialize_nested<O: OutputStream>(&self, field: u32, out: &mut O) -> io::Result<()> {
        (&**self).serialize_nested(field, out)
    }
}

impl Deserialize for String {
    fn deserialize<R: io::Read>(_: &mut InputStream<R>) -> io::Result<Self> {
        unimplemented!();
    }

    fn deserialize_nested<R: io::Read>(field: &mut Field<R>) -> io::Result<String> {
        match String::from_utf8(try!(field.read_bytes())) {
            Ok(s) => Ok(s),
            Err(_) => Err(unexpected_output("string not UTF-8 encoded"))
        }
    }
}

/*
 *
 * ===== Option =====
 *
 */

impl<T: Serialize> Serialize for Option<T> {
    fn serialize<O: OutputStream>(&self, _: &mut O) -> io::Result<()> {
        unimplemented!();
    }

    fn serialize_nested<O: OutputStream>(&self, field: u32, out: &mut O) -> io::Result<()> {
        if let Some(ref val) = *self {
            val.serialize_nested(field, out)
        } else {
            Ok(())
        }
    }
}

/*
 *
 * ===== Tuples =====
 *
 */

impl<T1, T2> Serialize for (T1, T2)
        where T1: Serialize,
              T2: Serialize {

    fn serialize<O: OutputStream>(&self, out: &mut O) -> io::Result<()> {
        try!(out.write(1, &self.0));
        try!(out.write(2, &self.1));
        Ok(())
    }
}

impl<T1, T2> Deserialize for (T1, T2)
        where T1: Deserialize,
              T2: Deserialize {

    fn deserialize<R: io::Read>(i: &mut InputStream<R>) -> io::Result<Self> {
        let mut a = None;
        let mut b = None;

        while let Some(mut f) = try!(i.read_field()) {
            match f.tag() {
                1 => a = Some(try!(f.read())),
                2 => b = Some(try!(f.read())),
                _ => try!(f.skip()),
            }
        }

        Ok((required!(a, "tuple::0"), required!(b, "tuple::1")))
    }
}

/*
 *
 * ===== Varint =====
 *
 */

/// Trait for values that can be serialized / deserialized as varints
pub trait Varint: Sized {
    #[doc(hidden)]
    fn wire_len(self) -> usize;

    #[doc(hidden)]
    fn write<W: io::Write>(self, dst: &mut W) -> io::Result<()>;

    #[doc(hidden)]
    fn read<R: io::Read>(src: &mut R) -> io::Result<Option<Self>>;
}

macro_rules! impl_unsigned {
    ($Ty:ty) => {
        impl Serialize for $Ty {
            fn serialize<O: OutputStream>(&self, _: &mut O) -> io::Result<()> {
                unimplemented!();
            }

            fn serialize_nested<O: OutputStream>(&self, field: u32, out: &mut O) -> io::Result<()> {
                out.write_varint(field, *self)
            }
        }

        impl Deserialize for $Ty {
            fn deserialize<R: io::Read>(i: &mut InputStream<R>) -> io::Result<Self> {
                match try!(i.read_varint()) {
                    Some(v) => Ok(v),
                    None => Err(eof()),
                }
            }

            fn deserialize_nested<R: io::Read>(field: &mut Field<R>) -> io::Result<Self> {
                field.read_varint()
            }
        }

        impl Varint for $Ty {
            #[inline]
            fn wire_len(self) -> usize {
                let mut num = self as u64;
                let mut len = 0;

                // Handle a common case
                if num & (!0 << 7) == 0 {
                    return 1;
                }

                let mut n = 2;

                if num & (!0 << 35) != 0 {
                    n += 4;
                    num >>= 28;
                }

                if num & (!0 << 21) != 0 {
                    n += 2;
                    num >> 14;
                }

                if num & (!0 << 14) != 0 {
                    n += 1;
                }

                len += n;
                len
            }

            #[inline]
            fn write<W: io::Write>(mut self, dst: &mut W) -> io::Result<()> {
                loop {
                    // Grab up to 7 bits of the number
                    let bits = (self & 0x7f) as u8;

                    // Shift the remaining bits
                    self >>= 7;

                    if self == 0 {
                        try!(dst.write_all(&[bits]));
                        return Ok(());
                    }

                    try!(dst.write_all(&[bits | 0x80]));
                }
            }

            #[inline]
            fn read<R: io::Read>(src: &mut R) -> io::Result<Option<$Ty>> {
                let mut ret = 0;
                let mut shift = 0;

                let mut buf = [0; 1];

                loop {
                    match src.read(&mut buf) {
                        Ok(0) => break,
                        Ok(_) => {
                            let byte = buf[0];
                            let bits = (byte & 0x7f) as $Ty;

                            // TODO: Handle overflow
                            ret |= bits << shift;
                            shift += 7;

                            if !has_msb(byte) {
                                return Ok(Some(ret));
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }

                match shift {
                    0 => Ok(None),
                    _ => Err(eof()),
                }
            }
        }
    }
}

impl_unsigned! { u8 }
impl_unsigned! { u16 }
impl_unsigned! { u32 }
impl_unsigned! { u64 }
impl_unsigned! { usize }

fn has_msb(byte: u8) -> bool {
    byte & 0x80 != 0
}

fn unexpected_output(desc: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, desc)
}

fn eof() -> io::Error {
    return unexpected_output("unexpected EOF");
}
