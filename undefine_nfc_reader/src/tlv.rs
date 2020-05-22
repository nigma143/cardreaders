use std::fmt;
use tlv_parser::tlv::Tlv;

pub struct TlvExtended {
    tlv: Tlv,
}

impl TlvExtended {
    pub fn new(tlv: Tlv) -> Self {
        TlvExtended { tlv }
    }
}

impl TlvExtended {
    fn fmt_ext(tlv: &Tlv, ident: &mut String, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}- {:02X}: ", &ident, tlv.tag())?;
        match tlv.val() {
            tlv_parser::tlv::Value::Val(val) => write!(f, "{:02X?}", val),
            tlv_parser::tlv::Value::TlvList(childs) => {
                writeln!(f)?;
                ident.push_str("  ");
                for child in childs {
                    Self::fmt_ext(child, ident, f)?;
                }
                Ok(())
            }
            tlv_parser::tlv::Value::Nothing => write!(f, ""),
        }
    }
}

impl fmt::Display for TlvExtended {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Self::fmt_ext(&self.tlv, &mut "".to_owned(), f)
    }
}
