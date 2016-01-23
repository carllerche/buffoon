use std::{cmp, io};

pub struct Take<T> {
    inner: T,
    limit: u64,
}

impl<T> Take<T> {
    pub fn new(inner: T, limit: u64) -> Take<T> {
        Take {
            inner: inner,
            limit: limit,
        }
    }

    /// Returns the number of bytes that can be read before this instance will
    /// return EOF.
    ///
    /// # Note
    ///
    /// This instance may reach EOF after reading fewer bytes than indicated by
    /// this method if the underlying `Read` instance reaches EOF.
    pub fn limit(&self) -> u64 { self.limit }

    pub fn set_limit(&mut self, limit: u64) { self.limit = limit }
}

impl<T: io::Read> io::Read for Take<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Don't call into inner reader at all at EOF because it may still block
        if self.limit == 0 {
            return Ok(0);
        }

        let max = cmp::min(buf.len() as u64, self.limit) as usize;
        let n = try!(self.inner.read(&mut buf[..max]));
        self.limit -= n as u64;
        Ok(n)
    }
}
