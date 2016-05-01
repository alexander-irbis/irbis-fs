use std::fmt;

use protocol::serde::{Encode, Parse, Parser, ParserError};


pub type TaskId = u64;

pub struct ContentId([u8; 64]);

// --------------------------------------------------------------------------------------------------------------------

impl ContentId {
    pub fn from_slice(slice: &[u8]) -> Self {
        ContentId(*slice_as_array!(slice, [u8; 64]).unwrap())
    }

    pub fn to_string(&self) -> String {
        (&self.0).iter().map(|x| format!("{:x}", x)).collect::<Vec<String>>().join("")
    }
}

impl Encode for ContentId {
    fn encode(self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl Parse for ContentId {
    fn parse_from(parser: &mut Parser) -> Result<ContentId, ParserError> {
        Ok(ContentId::from_slice(parser.next(64)))
    }
}

impl fmt::Debug for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ContentId ({})", self.to_string())
    }
}


// --------------------------------------------------------------------------------------------------------------------

