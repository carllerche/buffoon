#[derive(Debug)]
pub enum WireType {
    Varint = 0,
    SixtyFourBit = 1,
    LengthDelimited = 2,
    StartGroup = 3,
    EndGroup = 4,
    ThirtyTwoBit = 5
}

impl WireType {
    pub fn from_u32(val: u32) -> Option<WireType> {
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
