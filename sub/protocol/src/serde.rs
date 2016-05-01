
use std::cell::Cell;
use std::mem::transmute;
use std::mem::size_of_val;
use std::ops::AddAssign;
use std::str::{from_utf8, Utf8Error};


#[derive(Debug)]
pub struct Encoder {
    data: Vec<u8>,
}


#[derive(Debug)]
pub enum ParserError {
    // Not all data was parsed
    Incomplete,
    Overflow,
    Utf8Error(Utf8Error),
}

#[derive(Debug)]
pub struct Parser {
    data: Vec<u8>,
    position: usize,
}

// --------------------------------------------------------------------------------------------------------------------

impl Encoder {
    pub fn new() -> Self {
        Encoder { data: Vec::new() }
    }

    pub fn complete(self) -> Vec<u8> {
        self.data
    }

    pub fn extend_from_slice<'a>(&mut self, v: &'a [u8]) {
        self.data.extend_from_slice(v);
    }

    pub fn extend_from_vec(&mut self, v: Vec<u8>) {
        self.data.extend(v);
    }
}


pub trait Encode {
    fn encode(self) -> Vec<u8>;
}


impl <E: Encode> AddAssign<E> for Encoder {
    fn add_assign(&mut self, v: E) {
        self.extend_from_vec(v.encode());
    }
}


impl Encode for u8 { fn encode(self) -> Vec<u8> { encode_u8(self.clone()).to_vec() } }
impl Encode for u16 { fn encode(self) -> Vec<u8> { encode_u16(self.clone()).to_vec() } }
impl Encode for u32 { fn encode(self) -> Vec<u8> { encode_u32(self.clone()).to_vec() } }
impl Encode for u64 { fn encode(self) -> Vec<u8> { encode_u64(self.clone()).to_vec() } }
impl Encode for usize { fn encode(self) -> Vec<u8> { encode_usize(self.clone()).to_vec() } }
impl Encode for String { fn encode(self) -> Vec<u8> { encode_str(&self) } }


// --------------------------------------------------------------------------------------------------------------------


impl From<Utf8Error> for ParserError {
    fn from(err: Utf8Error) -> Self {
        ParserError::Utf8Error(err)
    }
}


//pub trait Decode: Into {
//    fn decode() {
//
//    }
//}
//
//
//pub struct Decoder(&Parser);
//
//
//impl Decode<u64> for Decoder {
//    fn into(self) -> u64 {
//        decode_u64(&self.0.next(8))
//    }
//}


impl Parser {
    pub fn new(data: Vec<u8>) -> Self {
        Parser {
            data: data,
            position: 0,
        }
    }

    pub fn next(&mut self, n: usize) -> &[u8] {
        self.position += n;
        &self.data[(self.position - n) .. self.position]
    }

    pub fn complete(self) -> Result<(), ParserError> {
        match self.position == self.data.len() {
            true => Ok(()),
            false => Err(ParserError::Incomplete)
        }
    }

//    pub fn decode<T: Decode>(&mut self) -> Decoder<T> {
//        Decoder<T>(&mut self)
//    }

//    pub fn u8(&mut self) -> Result<u8, ParserError> {
//        let v = decode_u8(&self.data[self.position..]);
//        self.position += size_of_val(&v);
//        Ok(v)
//    }
//
//    pub fn u16(&mut self) -> Result<u16, ParserError> {
//        let v = decode_u16(&self.data[self.position..]);
//        self.position += size_of_val(&v);
//        Ok(v)
//    }
//
//    pub fn u32(&mut self) -> Result<u32, ParserError> {
//        let v = decode_u32(&self.data[self.position..]);
//        self.position += size_of_val(&v);
//        Ok(v)
//    }
//
//    pub fn u64(&mut self) -> Result<u64, ParserError> {
//        let v = decode_u64(&self.data[self.position..]);
//        self.position += size_of_val(&v);
//        Ok(v)
//    }
//
//    pub fn usize(&mut self) -> Result<usize, ParserError> {
//        let v = decode_usize(&self.data[self.position..]);
//        self.position += size_of_val(&v);
//        Ok(v)
//    }
//
//    pub fn s16(&mut self) -> Result<String, ParserError> {
//        let (size, v) = try!(decode_s16(&self.data[self.position..]));
//        self.position += size;
//        Ok(v)
//    }
//
//    pub fn str(&mut self) -> Result<String, ParserError> {
//        let (size, v) = try!(decode_str(&self.data[self.position..]));
//        self.position += size;
//        Ok(v)
//    }
}


pub trait Parse: Sized {
    fn parse_from(parser: &mut Parser) -> Result<Self, ParserError>;
}


impl Parse for u8 { fn parse_from(parser: &mut Parser) -> Result<u8, ParserError> {Ok(decode_u8(parser.next(1)))} }
impl Parse for u16 { fn parse_from(parser: &mut Parser) -> Result<u16, ParserError> {Ok(decode_u16(parser.next(2)))} }
impl Parse for u32 { fn parse_from(parser: &mut Parser) -> Result<u32, ParserError> {Ok(decode_u32(parser.next(4)))} }
impl Parse for u64 { fn parse_from(parser: &mut Parser) -> Result<u64, ParserError> {Ok(decode_u64(parser.next(8)))} }
impl Parse for usize { fn parse_from(parser: &mut Parser) -> Result<usize, ParserError> {Ok(decode_usize(parser.next(8)))} }

impl Parse for String {
    fn parse_from(parser: &mut Parser) -> Result<String, ParserError> {
        let (size, v) = try!(decode_str(&parser.data[parser.position..]));
        parser.position += size;
        Ok(v)
    }
}





pub fn encode_u8(v: u8) -> [u8; 1] {
    unsafe { transmute::<u8, [u8; 1]>(v.to_be()) }
}

pub fn decode_u8(v: &[u8]) -> u8 {
    let mut buf = [0u8; 1];
    buf.copy_from_slice(&v[..1]);
    u8::from_be(unsafe { transmute::<[u8; 1], u8>(buf) })
}


pub fn encode_u16(v: u16) -> [u8; 2] {
    unsafe { transmute::<u16, [u8; 2]>(v.to_be()) }
}

pub fn decode_u16(v: &[u8]) -> u16 {
    let mut buf = [0u8; 2];
    buf.copy_from_slice(&v[..2]);
    u16::from_be(unsafe { transmute::<[u8; 2], u16>(buf) })
}


pub fn encode_u32(v: u32) -> [u8; 4] {
    unsafe { transmute::<u32, [u8; 4]>(v.to_be()) }
}

pub fn decode_u32(v: &[u8]) -> u32 {
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&v[..4]);
    u32::from_be(unsafe { transmute::<[u8; 4], u32>(buf) })
}


pub fn encode_u64(v: u64) -> [u8; 8] {
    unsafe { transmute::<u64, [u8; 8]>(v.to_be()) }
}

pub fn decode_u64(v: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&v[..8]);
    u64::from_be(unsafe { transmute::<[u8; 8], u64>(buf) })
}


pub fn encode_usize(v: usize) -> [u8; 8] {
    unsafe { transmute::<usize, [u8; 8]>(v.to_be()) }
}

pub fn decode_usize(v: &[u8]) -> usize {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&v[..8]);
    usize::from_be(unsafe { transmute::<[u8; 8], usize>(buf) })
}



pub fn encode_s16(v: &str) -> Vec<u8> {
    [&encode_u16(v.len() as u16), v.as_bytes()].concat()
}

pub fn decode_s16(v: &[u8]) -> Result<(usize, String), Utf8Error> {
    let end = decode_u16(&v[..2]) as usize + 2;
    let s = try!(from_utf8(&v[2 .. end])).to_owned();
    Ok((end, s))
}


pub fn encode_len(len: usize) -> Vec<u8> {
    let mut data: Vec<u8> =
        if len < 128 { [&encode_u8(len as u8)[..]].concat() }
        else { [&encode_u32(len as u32)[..]].concat() }
    ;

    if data.len() > 1 {
        data[0] |= 0b_1000_0000;
    }
    data
}

pub fn decode_len(v: &[u8]) -> (usize, usize) {
    if v[0] & 0b_1000_0000 == 0 {
        (1 as usize, decode_u8(v) as usize)
    } else {
        let mut data = [0u8; 4];
        data.copy_from_slice(v);
        data[0] &= 0b_0111_1111;
        (4 as usize, decode_u32(&data[..]) as usize)
    }
}


pub fn encode_str(v: &str) -> Vec<u8> {
    [
        &encode_len(v.len())[..],
        v.as_bytes()
    ].concat()
}

pub fn decode_str(v: &[u8]) -> Result<(usize, String), Utf8Error> {
    let (size, len) = decode_len(v);
    let s = try!(from_utf8(&v[size .. size + len])).to_owned();
    Ok((size + len, s))
}
