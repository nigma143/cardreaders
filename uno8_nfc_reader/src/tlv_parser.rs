use std::fmt;

use thiserror::Error;

pub type Tag = usize;
type Tags = Vec<Tag>;

#[derive(Error, Debug)]
pub enum TlvError {
    #[error("Too short input vector")]
    TruncatedTlv,

    #[error("Invalid length value")]
    InvalidLength,

    #[error("Invalid tag number")]
    InvalidTagNumber,

    #[error("Too short body: expected {}, found {}", expected, found)]
    TooShortBody { expected: usize, found: usize },

    #[error(
        "Tag number defines constructed TLV, but value is not Value::TlvList: {}",
        tag_number
    )]
    TlvListExpected { tag_number: usize },

    #[error(
        "Tag number defines primitive TLV, but value is not Value::Val: {}",
        tag_number
    )]
    ValExpected { tag_number: usize },

    #[error("Provided 'tag-path' have error")]
    TagPathError,

    #[error("Tag value parse error: {0}")]
    ParseTagValue(String),
}

#[derive(Debug)]
pub enum Value {
    TlvList(Vec<Tlv>),
    Val(Vec<u8>),
    Nothing,
}

impl Value {
    /// Returns size of value in bytes
    fn len(&self) -> usize {
        match *self {
            Value::TlvList(ref list) => list.iter().fold(0, |sum, x| sum + x.len()),
            Value::Val(ref v) => v.len(),
            Value::Nothing => 0,
        }
    }

    /// Returns true if Value is empty (len() == 0)
    pub fn is_empty(&self) -> bool {
        match *self {
            Value::TlvList(ref list) => list.is_empty(),
            Value::Val(ref v) => v.is_empty(),
            Value::Nothing => true,
        }
    }

    /// Returns bytes array that represents encoded-len
    ///
    /// Note: implements only definite form
    pub fn encode_len(&self) -> Vec<u8> {
        let len = self.len();
        if len <= 0x7f {
            return vec![len as u8];
        }

        let mut out: Vec<u8> = len
            .to_be_bytes()
            .iter()
            .skip_while(|&x| *x == 0)
            .cloned()
            .collect();

        let bytes = out.len() as u8;
        out.insert(0, 0x80 | bytes);
        out
    }
}

pub trait TagValue {
    type Value;

    fn new(val: Self::Value) -> Self;

    fn from_raw(raw: Vec<u8>) -> Result<Self, TlvError>
    where
        Self: Sized;

    fn bytes(&self) -> Vec<u8>;
}

#[derive(Debug)]
pub struct Tlv {
    tag: Tag,
    val: Value,
}

impl Tlv {
    /// Creates Tlv object
    ///
    /// # Examples:
    ///
    /// Create empty primitive TLV:
    ///
    /// ```
    /// # use tlv_parser::tlv::*;
    /// #
    /// let primitive_tlv = Tlv::new(0x01, Value::Nothing).unwrap();
    /// # let constructed_tlv = Tlv::new(0x21, Value::TlvList(vec![primitive_tlv])).unwrap();
    /// #
    /// # assert_eq!(constructed_tlv.to_vec(), vec![0x21, 0x02, 0x01, 0x00]);
    /// ```
    ///
    /// Create constructed TLV incapsulated primitive TLV:
    ///
    /// ```
    /// # use tlv_parser::tlv::*;
    /// #
    /// # let primitive_tlv = Tlv::new(0x01, Value::Nothing).unwrap();
    /// let constructed_tlv = Tlv::new(0x21, Value::TlvList(vec![primitive_tlv])).unwrap();
    /// #
    /// # assert_eq!(constructed_tlv.to_vec(), vec![0x21, 0x02, 0x01, 0x00]);
    /// ```
    pub fn new(tag: Tag, value: Value) -> Result<Tlv, TlvError> {
        let tlv = Tlv {
            tag,
            val: Value::Nothing,
        };
        match value {
            Value::TlvList(_) => {
                if tlv.is_primitive() {
                    Err(TlvError::ValExpected { tag_number: tag })?;
                }
            }
            Value::Val(_) => {
                if !tlv.is_primitive() {
                    Err(TlvError::TlvListExpected { tag_number: tag })?;
                }
            }
            _ => (),
        }

        Ok(Tlv { tag, val: value })
    }

    /// Returns tag number of TLV
    pub fn tag(&self) -> Tag {
        self.tag
    }

    /// Returns length of tag number
    ///
    /// # Examples
    ///
    /// ```
    /// # use tlv_parser::tlv::*;
    /// let tag_len = Tlv::new(0x01, Value::Nothing).unwrap().tag_len();
    /// assert_eq!(tag_len, 1);
    /// ```
    pub fn tag_len(&self) -> usize {
        let mut tag = self.tag;
        let mut len = 0;
        while tag != 0 {
            len += 1;
            tag >>= 8;
        }

        if len == 0 {
            len + 1
        } else {
            len
        }
    }

    /// Returns size of TLV-string in bytes
    ///
    /// # Examples
    ///
    /// ```
    /// # use tlv_parser::tlv::*;
    /// let tlv_len = Tlv::new(0x01, Value::Val(vec![0x02, 0x03])).unwrap().len();
    /// assert_eq!(tlv_len, 4);
    /// ```
    pub fn len(&self) -> usize {
        self.tag_len() + self.val.encode_len().len() + self.val.len()
    }

    /// Returns true if Value of TLV is empty
    pub fn is_empty(&self) -> bool {
        self.val.is_empty()
    }

    /// Returns value if TLV
    pub fn val(&self) -> &Value {
        &self.val
    }

    /// Returns TLV-encoded array of bytes
    ///
    /// # Examples
    ///
    /// ```
    /// # use tlv_parser::tlv::*;
    /// let tlv = Tlv::new(0x21,
    ///     Value::TlvList(vec![
    ///         Tlv::new( 0x01, Value::Val(vec![0xA1, 0xA2])).unwrap(),
    ///         Tlv::new( 0x02, Value::Val(vec![0xB1, 0xB2])).unwrap()])).unwrap();
    /// assert_eq!(tlv.to_vec(), vec![0x21, 0x08, 0x01, 0x02, 0xA1, 0xA2, 0x02, 0x02, 0xB1, 0xB2]);
    /// ```
    pub fn to_vec(&self) -> Vec<u8> {
        let mut out: Vec<u8> = (self.tag as u64)
            .to_be_bytes()
            .iter()
            .skip_while(|&&x| x == 0)
            .cloned()
            .collect();

        out.append(&mut self.val.encode_len());

        match self.val {
            Value::TlvList(ref list) => {
                for x in list.iter() {
                    out.append(&mut x.to_vec());
                }
            }
            Value::Val(ref v) => out.extend_from_slice(v),
            Value::Nothing => (),
        };

        out
    }

    /// Parses string like "6F / A5" into Tags
    fn get_path(path: &str) -> Result<Tags, TlvError> {
        let tags: Result<Vec<_>, TlvError> = path
            .chars()
            .filter(|&x| x.is_digit(16) || x == '/')
            .collect::<String>()
            .split('/')
            .map(|x| usize::from_str_radix(&x, 16).map_err(|_| TlvError::TagPathError))
            .collect();

        tags
    }

    /// Returns value of TLV
    ///
    /// # Example
    ///
    /// ```
    /// # use tlv_parser::tlv::*;
    /// let tlv = Tlv::from_vec(&[0x6F, 0x09, 0xA5, 0x07, 0xBF, 0x0C, 0x04, 0xDF, 0x7F, 0x01, 0x55])
    ///     .unwrap();
    /// if let &Value::Val(ref v) = tlv.find_val("6F / A5 / BF0C / DF7F").unwrap() {
    ///     assert_eq!(*v, vec![0x55]);
    /// }
    /// ```
    pub fn find_val(&self, path: &str) -> Option<&Value> {
        let path = match Tlv::get_path(path) {
            Ok(x) => x,
            _ => return None,
        };

        if path.is_empty() {
            return None;
        }

        if path[0] != self.tag {
            return None;
        }

        if path.len() == 1 {
            return Some(&self.val);
        }

        let mut tlv = self;
        let mut i = 1;

        for tag in path.iter().skip(1) {
            i += 1;

            if let Value::TlvList(ref list) = tlv.val {
                for subtag in list {
                    if *tag != subtag.tag {
                        continue;
                    }

                    if path.len() == i {
                        return Some(&subtag.val);
                    }

                    tlv = subtag;
                    break;
                }
            } else {
                return None;
            }
        }

        None
    }

    /// Reads out tag number
    fn read_tag(iter: &mut ExactSizeIterator<Item = &u8>) -> Result<Tag, TlvError> {
        let mut tag: usize;

        let first: u8 = iter.next().cloned().ok_or_else(|| TlvError::TruncatedTlv)?;
        tag = first as usize;

        if first & 0x1F == 0x1F {
            // long form - find the end
            for x in &mut *iter {
                tag = tag
                    .checked_shl(8)
                    .ok_or_else(|| TlvError::InvalidTagNumber)?;

                tag |= *x as usize;

                if *x & 0x80 == 0 {
                    break;
                }
            }
        }

        if tag == 0 {
            return Err(TlvError::InvalidTagNumber);
        }

        Ok(tag)
    }

    /// Reads out TLV value's length
    fn read_len(iter: &mut ExactSizeIterator<Item = &u8>) -> Result<usize, TlvError> {
        let mut len: usize;
        len = *iter.next().ok_or_else(|| TlvError::TruncatedTlv)? as usize;

        if len & 0x80 != 0 {
            let octet_num = len & 0x7F;

            len = 0;
            for x in iter.take(octet_num) {
                len = len.checked_shl(8).ok_or_else(|| TlvError::InvalidLength)?;

                len |= *x as usize;
            }
        }

        let remain = iter.len();
        if remain < len {
            Err(TlvError::TooShortBody {
                expected: len,
                found: remain,
            })?;
        }

        Ok(len)
    }

    /// Returns true if TLV is primitive
    pub fn is_primitive(&self) -> bool {
        let mask = 0x20 << ((self.tag_len() - 1) * 8);
        self.tag & mask != mask
    }

    /// Initializes Tlv object iterator of Vec<u8>
    fn from_iter(iter: &mut ExactSizeIterator<Item = &u8>) -> Result<Tlv, TlvError> {
        let tag = Tlv::read_tag(iter)?;
        let len = Tlv::read_len(iter)?;

        let val = &mut iter.take(len);

        let mut tlv = Tlv {
            tag,
            val: Value::Nothing,
        };

        if tlv.is_primitive() {
            tlv.val = Value::Val(val.cloned().collect());
            return Ok(tlv);
        }

        tlv.val = Value::TlvList(vec![]);

        while val.size_hint().0 != 0 {
            if let Value::TlvList(ref mut children) = tlv.val {
                children.push(Tlv::from_iter(val)?);
            }
        }

        Ok(tlv)
    }

    /// Initializes Tlv object from [u8] slice
    ///
    /// # Examples
    ///
    /// ```
    /// # use tlv_parser::tlv::*;
    /// let tlv = Tlv::from_vec(&[0x01, 0x00]).unwrap();
    /// assert_eq!(tlv.tag(), 0x01);
    /// assert_eq!(tlv.tag_len(), 0x01);
    /// assert_eq!(tlv.len(), 0x02);
    /// ```
    pub fn from_vec(slice: &[u8]) -> Result<Tlv, TlvError> {
        let iter = &mut slice.iter();
        Tlv::from_iter(iter)
    }

    fn display_write(tlv: &Tlv, ident: &mut String, output: &mut String) {
        output.push_str(&format!("{}- {:02X}: ", &ident, tlv.tag()));
        match tlv.val() {
            Value::Val(val) => output.push_str(&format!("{:02X?}", val)),
            Value::TlvList(childs) => {
                output.push_str("\n");
                ident.push_str("  ");
                for child in childs {
                    Self::display_write(&child, ident, output);
                }
            }
            Value::Nothing => output.push_str(""),
        }
    }
}

impl Tlv {
    pub fn new_spec(tag: usize, value: impl TagValue) -> Result<Self, TlvError> {
        Tlv::new(tag, Value::Val(value.bytes()))
    }

    pub fn child(&self, tag: usize) -> Result<&Self, TlvError> {
        match self.val() {
            Value::TlvList(childs) => match childs.iter().find(|x| x.tag == tag) {
                Some(tlv) => Ok(tlv),
                None => Err(TlvError::TagPathError),
            },
            _ => Err(TlvError::TlvListExpected { tag_number: tag }),
        }
    }
}

impl fmt::Display for Tlv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = String::new();
        Self::display_write(self, &mut "".to_owned(), &mut output);
        f.write_str(&output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_vec_test() {
        // simple two bytes TLV
        let input: Vec<u8> = vec![0x01, 0x02, 0x00, 0x00];
        assert_eq!(Tlv::from_vec(&input).unwrap().to_vec(), input);

        // TLV with two bytes tag
        let input: Vec<u8> = vec![0x9F, 0x02, 0x02, 0x00, 0x00];
        assert_eq!(Tlv::from_vec(&input).unwrap().to_vec(), input);

        // TLV with two bytes length
        let mut input: Vec<u8> = vec![0x9F, 0x02, 0x81, 0x80];
        input.extend_from_slice(&[0; 0x80]);
        assert_eq!(Tlv::from_vec(&input).unwrap().to_vec(), input);

        // TLV with four bytes tag number
        let input: Vec<u8> = vec![0x5f, 0xc8, 0x80, 0x01, 0x02, 0x01, 0x02];
        assert_eq!(Tlv::from_vec(&input).unwrap().to_vec(), input);
    }

    #[test]
    fn to_vec_test() {
        let tlv = Tlv {
            tag: 0x01,
            val: Value::Val(vec![0]),
        };

        assert_eq!(tlv.to_vec(), vec![0x01, 0x01, 0x00]);

        let tlv = Tlv {
            tag: 0x01,
            val: Value::Val(vec![0; 127]),
        };

        assert_eq!(&tlv.to_vec()[0..3], [0x01, 0x7F, 0x00]);

        let tlv = Tlv {
            tag: 0x01,
            val: Value::Val(vec![0; 255]),
        };

        assert_eq!(&tlv.to_vec()[0..4], [0x01, 0x81, 0xFF, 0x00]);

        let tlv = Tlv {
            tag: 0x02,
            val: Value::Val(vec![0; 256]),
        };

        assert_eq!(&tlv.to_vec()[0..4], [0x02, 0x82, 0x01, 0x00]);

        let tlv = Tlv {
            tag: 0x03,
            val: Value::Val(vec![0; 0xffff01]),
        };

        assert_eq!(&tlv.to_vec()[0..5], [0x03, 0x83, 0xFF, 0xFF, 0x01]);
    }

    #[test]
    fn find_val_test() {
        let input: Vec<u8> = vec![0x21, 0x05, 0x22, 0x03, 0x03, 0x01, 0xaa];
        let tlv = Tlv::from_vec(&input).unwrap();

        if let Some(&Value::Val(ref val)) = tlv.find_val("21 / 22 / 03") {
            assert_eq!(*val, vec![0xaa]);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn tag_len_test() {
        let tlv1 = Tlv {
            tag: 0x03,
            val: Value::Nothing,
        };
        let tlv2 = Tlv {
            tag: 0x0303,
            val: Value::Nothing,
        };
        let tlv3 = Tlv {
            tag: 0x030303,
            val: Value::Nothing,
        };
        let tlv4 = Tlv {
            tag: 0x03030303,
            val: Value::Nothing,
        };

        assert_eq!(tlv1.tag_len(), 1);
        assert_eq!(tlv2.tag_len(), 2);
        assert_eq!(tlv3.tag_len(), 3);
        assert_eq!(tlv4.tag_len(), 4);
    }
}
