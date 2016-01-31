#[macro_use]
extern crate buffoon;
extern crate env_logger;

use buffoon::*;
use std::io;

#[derive(Debug, PartialEq)]
struct Root {
    foo: Foo,
}

struct Empty1;
struct Empty2;

#[derive(Debug, PartialEq)]
struct Foo {
    val: u32,
}

impl Serialize for Root {
    fn serialize<O: OutputStream>(&self, out: &mut O) -> io::Result<()> {
        try!(out.write(1, &Empty1));
        try!(out.write(2, &self.foo));
        Ok(())
    }
}

impl Deserialize for Root {
    fn deserialize<R: io::Read>(i: &mut InputStream<R>) -> io::Result<Root> {
        let mut foo = None;

        while let Some(f) = try!(i.read_field()) {
            match f.tag() {
                1 => try!(f.skip()),
                2 => foo = Some(try!(f.read())),
                _ => panic!("unexpected field; field={:?}", f.tag()),
            }
        }

        Ok(Root { foo: required!(foo) })
    }
}

impl Serialize for Empty1 {
    fn serialize<O: OutputStream>(&self, out: &mut O) -> io::Result<()> {
        out.write(1, &Empty2)
    }
}

impl Serialize for Empty2 {
    fn serialize<O: OutputStream>(&self, _: &mut O) -> io::Result<()> {
        Ok(())
    }
}

impl Serialize for Foo {
    fn serialize<O: OutputStream>(&self, out: &mut O) -> io::Result<()> {
        out.write(1, &self.val)
    }
}

impl Deserialize for Foo {
    fn deserialize<R: io::Read>(i: &mut InputStream<R>) -> io::Result<Foo> {
        let mut val = None;

        while let Some(f) = try!(i.read_field()) {
            match f.tag() {
                1 => val = Some(try!(f.read())),
                _ => panic!("unexpected field"),
            }
        }

        Ok(Foo { val: required!(val) })
    }
}

#[test]
pub fn test_serializing_nested_empty() {
    let _ = env_logger::init();

    let root = Root { foo: Foo { val: 123 } };
    let bytes = buffoon::serialize(&root).unwrap();
    let root2 = buffoon::deserialize(io::Cursor::new(&bytes)).unwrap();

    assert_eq!(root, root2);
}
