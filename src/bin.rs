use std::convert::TryInto;

#[inline]
pub fn first_nibble(b: u16) -> u16 {
    b >> 12
}

#[inline]
pub fn second_nibble(b: u16) -> u16 {
    b >> 8 & 0xf
}

#[inline]
pub fn third_nibble(b: u16) -> u16 {
    b >> 4 & 0xf
}

#[inline]
pub fn fourth_nibble(b: u16) -> u16 {
    b & 0xf
}

#[inline]
pub fn lower_half(b: u16) -> u16 {
    b & 0x00ff
}

#[inline]
pub fn lower_three(b: u16) -> u16 {
    b & 0x0fff
}

#[inline]
pub fn to_byte(b: u16) -> u8 {
    b.try_into().unwrap()
}

#[inline]
pub fn to_usize(b: u16) -> usize {
    b.try_into().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_VALUE: u16 = 0xabcd;

    #[test]
    fn test_first_nibble() {
        assert_eq!(first_nibble(TEST_VALUE), 0xa);
    }

    #[test]
    fn test_second_nibble() {
        assert_eq!(second_nibble(TEST_VALUE), 0xb);
    }

    #[test]
    fn test_third_nibble() {
        assert_eq!(third_nibble(TEST_VALUE), 0xc);
    }

    #[test]
    fn test_fourth_nibble() {
        assert_eq!(fourth_nibble(TEST_VALUE), 0xd);
    }

    #[test]
    fn test_lower_half() {
        assert_eq!(lower_half(TEST_VALUE), 0xcd);
    }

    #[test]
    fn test_lower_three() {
        assert_eq!(lower_three(TEST_VALUE), 0xbcd);
    }
}
