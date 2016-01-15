use {Serialize, Varint};
use wire_type::WireType;
use std::io;

pub trait OutputStream {

    /*
     *
     * ===== Public =====
     *
     */

    /// Writes a nested message with the specified field number
    fn write<T: ?Sized + Serialize>(&mut self, field: u32, val: &T) -> io::Result<()>;

    /// Write a repeated message field
    fn write_repeated<T, I>(&mut self, field: u32, msgs: I) -> io::Result<()>
            where T: Serialize,
                  I: IntoIterator<Item=T> {
        for msg in msgs {
            try!(self.write(field, &msg));
        }

        Ok(())
    }

    /// Write a list of repeated varints in packed format
    fn write_packed<T, I>(&mut self, field: u32, vals: I) -> io::Result<()>
            where T: Varint,
                  I: IntoIterator<Item=T>;

    /*
     *
     * ===== Private =====
     *
     */

    #[doc(hidden)]
    fn write_nested<T: ?Sized + Serialize>(&mut self, field: u32, val: &T) -> io::Result<()>;

    #[doc(hidden)]
    fn write_raw_bytes(&mut self, bytes: &[u8]) -> io::Result<()>;

    #[doc(hidden)]
    fn write_varint<T: Varint>(&mut self, field: u32, val: T) -> io::Result<()>;

    #[doc(hidden)]
    fn write_raw_varint<T: Varint>(&mut self, val: T) -> io::Result<()>;

    #[doc(hidden)]
    fn write_bytes(&mut self, field: u32, val: &[u8]) -> io::Result<()>;
}

// Interal helper
pub fn write_head<O: OutputStream>(out: &mut O, field: u32, wire_type: WireType) -> io::Result<()> {
    // TODO: Handle overflow
    let bits = (field << 3) | (wire_type as u32);
    try!(out.write_raw_varint(bits));
    Ok(())
}
