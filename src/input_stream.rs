use {Deserialize, Varint};
use wire_type::WireType;
use std::{fmt, usize};
use std::io::{self, Read};
use std::marker::PhantomData;

/*
 *
 * ===== InputStream =====
 *
 */

/// `InputStream` allows reading Protocol Buffers encoded data off of a stream.
pub struct InputStream<R> {
    reader: R
}

pub fn from<R: Read>(read: R) -> InputStream<R> {
    InputStream::from(read)
}

impl<R: Read> InputStream<R> {
    fn from(reader: R) -> InputStream<R> {
        InputStream { reader: reader }
    }

    /// Reads the a field header and returns a `Field` which allows reading the
    /// field data.
    pub fn read_field(&mut self) -> io::Result<Option<Field<R>>> {
        // Read the header byte. In this case, EOF errors are OK as they signify
        // that there is no field to read
        let head = match self.read_usize() {
            Ok(Some(h)) => h,
            Ok(None) => return Ok(None),
            Err(e) => return Err(e),
        };

        // Extract the type of the field
        let wire_type = match WireType::from_usize(head & 0x7) {
            Some(res) => res,
            None => return Err(unexpected_output("invalid wire type"))
        };

        Ok(Some(Field {
            input: self,
            tag: head >> 3,
            wire_type: wire_type
        }))
    }

    /// Reads an unsigned varint as `usize`.
    ///
    /// If at EOF before reading the first byte, returns Ok(None).
    fn read_usize(&mut self) -> io::Result<Option<usize>> {
        if let Some(num) = try!(self.read_unsigned_varint()) {
            if num > (usize::MAX as u64) {
                return Err(unexpected_output("requested value could not fit in usize"));
            }

            return Ok(Some(num as usize));
        }

        Ok(None)
    }

    /// Read an unsigned varint as `u64`.
    ///
    /// If at EOF before reading the first byte, returns Ok(None).
    fn read_u64(&mut self) -> io::Result<Option<u64>> {
        self.read_unsigned_varint()
    }

    /// Read an unsigned varint as `u64`.
    ///
    /// If at EOF before reading the first byte, returns Ok(None).
    fn read_unsigned_varint(&mut self) -> io::Result<Option<u64>> {
        let mut ret: u64 = 0;
        let mut shift = 0;

        while let Some(byte) = try!(self.read_byte()) {
            let bits = (byte & 0x7f) as u64;

            ret |= bits << shift;
            shift += 7;

            if !has_msb(byte) {
                return Ok(Some(ret));
            }
        }

        match shift {
            0 => Ok(None),
            _ => Err(eof()),
        }
    }

    /// Reads a length delimited field and returns the data as `Vec<u8>`
    fn read_length_delimited(&mut self) -> io::Result<Option<Vec<u8>>> {
        if let Some(len) = try!(self.read_usize()) {
            return self.read_exact(len).map(|ret| Some(ret));
        }

        Ok(None)
    }

    /// Skips the current field
    fn skip(&mut self, n: usize) -> io::Result<usize> {
        let mut i = 0;
        // Yes this is a terrible implementation, but something better depends on:
        // https://github.com/rust-lang/rust/issues/13989
        //
        // TODO: Consider using a &mut [u8]
        while i < n {
            if let None = try!(self.read_byte()) {
                return Ok(i);
            }

            i += 1;
        }

        Ok(i)
    }

    /// Read exactly `len` bytes and return the data read as `Vec<u8>`
    fn read_exact(&mut self, len: usize) -> io::Result<Vec<u8>> {
        use std::slice;

        let mut ret = Vec::with_capacity(len);

        unsafe {
            let mut buf = slice::from_raw_parts_mut(ret.as_mut_ptr(), len);
            let mut off = 0;

            while off < len {
                let cnt = try!(self.reader.read(&mut buf[off..]));

                if cnt == 0 {
                    return Err(eof());
                }

                off += cnt;
            }

            ret.set_len(len);
        }

        Ok(ret)
    }

    /// Reads and deserializes a nested message.
    fn read_message<T: Deserialize>(&mut self) -> io::Result<Option<T>> {
        if let Some(len) = try!(self.read_u64()) {
            let mut input = InputStream::from((&mut self.reader).take(len));
            return T::deserialize(&mut input).map(Some);
        }

        Ok(None)
    }

    #[inline]
    fn read_byte(&mut self) -> io::Result<Option<u8>> {
        let mut buf = [0; 1];

        if 1 == try!(self.reader.read(&mut buf)) {
            return Ok(Some(buf[0]));
        }

        Ok(None)
    }
}

/*
 *
 * ===== Field =====
 *
 */

pub struct Field<'a, R: 'a> {
    input: &'a mut InputStream<R>,
    tag: usize,
    wire_type: WireType
}

impl<'a, R: Read> Field<'a, R> {
    /// Get the field tag
    pub fn tag(&self) -> usize {
        self.tag
    }

    /// Skip the current field
    pub fn skip(&mut self) -> io::Result<()> {
        match self.wire_type {
            WireType::Varint => {
                if let Some(_) = try!(self.input.read_unsigned_varint()) {
                    return Ok(());
                }

                Err(eof())
            }
            WireType::SixtyFourBit => unimplemented!(),
            WireType::LengthDelimited => {
                if let Some(len) = try!(self.input.read_usize()) {
                    if len == try!(self.input.skip(len)) {
                        return Ok(());
                    }
                }

                Err(eof())
            }
            WireType::StartGroup => unimplemented!(),
            WireType::EndGroup => unimplemented!(),
            WireType::ThirtyTwoBit => unimplemented!()
        }
    }

    pub fn read<T: Deserialize>(&mut self) -> io::Result<T> {
        T::deserialize_nested(self)
    }

    pub fn read_packed<T: Varint>(&mut self) -> io::Result<Varints<T, R>> {
        match self.wire_type {
            WireType::LengthDelimited => {
                let len = try!(self.input.read_u64()).unwrap_or(0);
                let input = InputStream::from((&mut self.input.reader).take(len));
                Ok(Varints {
                    input: input,
                    phantom: PhantomData,
                })

            }
            _ => Err(unexpected_output("field type was not length delimited")),
        }
    }

    #[doc(hidden)]
    pub fn read_nested<T: Deserialize>(&mut self) -> io::Result<T> {
        match self.wire_type {
            WireType::LengthDelimited => {
                if let Some(val) = try!(self.input.read_message()) {
                    return Ok(val);
                }

                Err(eof())
            }
            _ => Err(unexpected_output("field type was not length delimited")),
        }
    }

    #[doc(hidden)]
    pub fn read_varint<T: Varint>(&mut self) -> io::Result<T> {
        match self.wire_type {
            WireType::Varint => {
                if let Some(val) = try!(T::read(&mut self.input.reader)) {
                    return Ok(val);
                }

                Err(eof())
            }
            _ => Err(unexpected_output("field type was not varint"))
        }
    }

    #[doc(hidden)]
    pub fn read_bytes(&mut self) -> io::Result<Vec<u8>> {
        match self.wire_type {
            WireType::LengthDelimited => {
                if let Some(val) = try!(self.input.read_length_delimited()) {
                    return Ok(val);
                }

                Err(eof())
            }
            _ => Err(unexpected_output("field type was not length delimited"))
        }
    }
}

impl<'a, R> fmt::Debug for Field<'a, R> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Field(tag={:?}; wire-type={:?})", self.tag, self.wire_type)
    }
}

/*
 *
 * ===== Varints =====
 *
 */

pub struct Varints<'a, T: Varint, R: 'a> {
    input: InputStream<io::Take<&'a mut R>>,
    phantom: PhantomData<T>,
}

impl<'a, T: Varint, R: 'a + io::Read> Iterator for Varints<'a, T, R> {
    type Item = io::Result<T>;

    fn next(&mut self) -> Option<io::Result<T>> {
        match T::read(&mut self.input.reader) {
            Ok(Some(v)) => Some(Ok(v)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

/*
 *
 * ===== Misc =====
 *
 */

fn has_msb(byte: u8) -> bool {
    byte & 0x80 != 0
}

fn unexpected_output(desc: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, desc)
}

fn eof() -> io::Error {
    return unexpected_output("unexpected EOF");
}

#[cfg(test)]
mod test {
    use std::io::Cursor;
    use super::InputStream;

    #[test]
    pub fn test_reading_empty_stream() {
        with_input_stream(&[], |i| {
            assert!(i.read_field().unwrap().is_none());
        });
    }
    #[test]
    pub fn test_reading_string() {
        with_input_stream(b"\x0A\x04zomg", |i| {
            {
                let mut f = i.read_field().unwrap().unwrap();
                assert_eq!(f.tag(), 1);
                assert_eq!(f.read::<String>().unwrap(), "zomg");
            }

            assert!(i.read_field().unwrap().is_none());
        });
    }

    #[test]
    pub fn test_reading_single_byte_usize() {
        with_input_stream(b"\x00\x08", |i| {
            {
                let mut f = i.read_field().unwrap().unwrap();
                assert_eq!(f.tag(), 0);
                assert_eq!(f.read::<u64>().unwrap(), 8);
            }

            assert!(i.read_field().unwrap().is_none());
        });
    }

    #[test]
    pub fn test_reading_multi_byte_usize() {
        with_input_stream(b"\x00\x92\x0C", |i| {
            {
                let mut f = i.read_field().unwrap().unwrap();
                assert_eq!(f.tag(), 0);
                assert_eq!(f.read::<u64>().unwrap(), 1554);
            }

            assert!(i.read_field().unwrap().is_none());
        });
    }

    #[test]
    pub fn test_reading_sequential_fields() {
        with_input_stream(b"\x00\x08\x0A\x04zomg\x12\x03lol", |i| {
            {
                let mut f = i.read_field().unwrap().unwrap();
                assert_eq!(f.tag(), 0);
                assert_eq!(f.read::<u64>().unwrap(), 8);
            }

            {
                let mut f = i.read_field().unwrap().unwrap();
                assert_eq!(f.tag(), 1);
                assert_eq!(f.read::<String>().unwrap(), "zomg");
            }

            {
                let mut f = i.read_field().unwrap().unwrap();
                assert_eq!(f.tag(), 2);
                assert_eq!(f.read::<String>().unwrap(), "lol");
            }

            assert!(i.read_field().unwrap().is_none());
        });
    }

    #[test]
    pub fn test_skipping_string_field() {
        with_input_stream(b"\x00\x08\x0A\x04zomg\x12\x03lol", |i| {
            i.read_field().unwrap().unwrap().skip().unwrap();

            {
                let mut f = i.read_field().unwrap().unwrap();
                assert_eq!(f.tag(), 1);
                assert_eq!(f.read::<String>().unwrap(), "zomg");
            }

            i.read_field().unwrap().unwrap().skip().unwrap();

            assert!(i.read_field().unwrap().is_none());
        });
    }

    #[test]
    pub fn test_reading_multi_byte_tag_field() {
        with_input_stream(b"\x92\x01\x04zomg", |i| {
            {
                let mut f = i.read_field().unwrap().unwrap();
                assert_eq!(f.tag(), 18);
                assert_eq!(f.read::<String>().unwrap(), "zomg");
            }

            assert!(i.read_field().unwrap().is_none());
        });
    }

    #[test]
    pub fn test_reading_twice_from_field() {
        with_input_stream(b"\x92\x01\x04zomg\x92\x01\x04zomg", |i| {
            {
                let mut f = i.read_field().unwrap().unwrap();
                f.read::<String>().unwrap();

                assert!(f.read::<String>().is_err());
            }
        });
    }

    #[test]
    pub fn test_reading_incorrect_type_from_field() {
        with_input_stream(b"\x92\x01\x04zomg", |i| {
            {
                let mut f = i.read_field().unwrap().unwrap();
                assert!(f.read::<u64>().is_err());
            }
        });
    }

    #[test]
    pub fn test_reading_packed_varints() {
        with_input_stream(b"\x22\x06\x03\x8e\x02\x9e\xa7\x05", |i| {
            let mut f = i.read_field().unwrap().unwrap();
            assert_eq!(f.tag(), 4);

            let nums: Vec<u64> = f.read_packed().unwrap().map(Result::unwrap).collect();
            assert_eq!(nums, [3, 270, 86942]);
        })
    }

    fn with_input_stream<F: FnOnce(&mut InputStream<Cursor<&[u8]>>)>(bytes: &[u8], action: F) {
        let mut input = InputStream::from(Cursor::new(bytes));
        action(&mut input)
    }
}
