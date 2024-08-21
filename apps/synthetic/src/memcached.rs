use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use rand::distributions::{Exp, IndependentSample};
use rand::{Rng, ThreadRng};
use std::io;
use std::io::{Error, ErrorKind, Read};

use super::Distribution;
use Connection;
use Packet;
use Transport;

/** Packet code from https://github.com/aisk/rust-memcache **/

#[allow(dead_code)]
enum Opcode {
    Get = 0x00,
    Set = 0x01,
    Add = 0x02,
    Replace = 0x03,
    Delete = 0x04,
    Increment = 0x05,
    Decrement = 0x06,
    Flush = 0x08,
    Noop = 0x0a,
    Version = 0x0b,
    GetKQ = 0x0d,
    Append = 0x0e,
    Prepend = 0x0f,
    Touch = 0x1c,
}

enum Magic {
    Request = 0x80,
    Response = 0x81,
}

#[allow(dead_code)]
enum ResponseStatus {
    NoError = 0x00,
    KeyNotFound = 0x01,
    KeyExists = 0x02,
    ValueTooLarge = 0x03,
    InvalidArguments = 0x04,
}

#[derive(Debug, Default)]
struct PacketHeader {
    pub magic: u8,
    pub opcode: u8,
    pub key_length: u16,
    pub extras_length: u8,
    pub data_type: u8,
    pub vbucket_id_or_status: u16,
    pub total_body_length: u32,
    pub opaque: u32,
    pub cas: u64,
}

impl PacketHeader {
    fn write<W: io::Write>(self, writer: &mut W) -> io::Result<()> {
        writer.write_u8(self.magic)?;
        writer.write_u8(self.opcode)?;
        writer.write_u16::<BigEndian>(self.key_length)?;
        writer.write_u8(self.extras_length)?;
        writer.write_u8(self.data_type)?;
        writer.write_u16::<BigEndian>(self.vbucket_id_or_status)?;
        writer.write_u32::<BigEndian>(self.total_body_length)?;
        writer.write_u32::<BigEndian>(self.opaque)?;
        writer.write_u64::<BigEndian>(self.cas)?;
        return Ok(());
    }

    fn read<R: io::Read>(reader: &mut R) -> io::Result<PacketHeader> {
        let magic = reader.read_u8()?;
        if magic != Magic::Response as u8 {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Bad magic number in response header: {}", magic),
            ));
        }
        let header = PacketHeader {
            magic: magic,
            opcode: reader.read_u8()?,
            key_length: reader.read_u16::<BigEndian>()?,
            extras_length: reader.read_u8()?,
            data_type: reader.read_u8()?,
            vbucket_id_or_status: reader.read_u16::<BigEndian>()?,
            total_body_length: reader.read_u32::<BigEndian>()?,
            opaque: reader.read_u32::<BigEndian>()?,
            cas: reader.read_u64::<BigEndian>()?,
        };
        return Ok(header);
    }
}

pub const NVALUES: usize = 100000;
// USR
static PCT_SET: u64 = 2; // out of 1000
static VALUE_SIZE: usize = 2;
static KEY_SIZE: usize = 20;

// ETC
static ETC_PCT_SET: u64 = 30; // out of 1000
static ETC_KEY_DISTR: Distribution = Distribution::GEV(30.7984, 8.20449, 0.078688);
static mut ETC_KEY_PRELOAD: [usize; NVALUES] = [0; NVALUES];
static ETC_VALUE_DISTR1: [(f64, usize); 15] = [
    (0.00536, 0),
    (0.00047, 1),
    (0.17820, 2),
    (0.09239, 3),
    (0.00018, 4),
    (0.02740, 5),
    (0.00065, 6),
    (0.00606, 7),
    (0.00023, 8),
    (0.00837, 9),
    (0.00837, 10),
    (0.08989, 11),
    (0.00092, 12),
    (0.00326, 13),
    (0.01980, 14),
];
static ETC_VALUE_DISTR2: Distribution = Distribution::GPerato(15.0, 214.476, 0.348238);

#[inline(always)]
fn write_key(buf: &mut Vec<u8>, key: u64, key_size: usize) {
    let mut pushed = 0;
    let mut k = key;
    loop {
        buf.push(48 + (k % 10) as u8);
        k /= 10;
        pushed += 1;
        if k == 0 {
            break;
        }
    }
    for _ in pushed..key_size {
        buf.push('A' as u8);
    }
}

static UDP_HEADER: &'static [u8] = &[0, 0, 0, 0, 0, 1, 0, 0];

#[derive(Copy, Clone, Debug)]
pub struct MemcachedProtocol;

impl MemcachedProtocol {
    pub fn usr_set_request(key: u64, opaque: u32, buf: &mut Vec<u8>, tport: Transport) {
        if let Transport::Udp = tport {
            buf.extend_from_slice(UDP_HEADER);
        }

        PacketHeader {
            magic: Magic::Request as u8,
            opcode: Opcode::Set as u8,
            key_length: KEY_SIZE as u16,
            extras_length: 8,
            total_body_length: (8 + KEY_SIZE + VALUE_SIZE) as u32,
            opaque,
            ..Default::default()
        }
        .write(buf)
        .unwrap();

        buf.write_u64::<BigEndian>(0).unwrap();

        write_key(buf, key, KEY_SIZE);

        for i in 0..VALUE_SIZE {
            buf.push((((key * i as u64) >> (i % 4)) & 0xff) as u8);
        }
    }

    pub fn gen_usr_request(i: usize, p: &Packet, buf: &mut Vec<u8>, tport: Transport) {
        // Use first 32 bits of randomness to determine if this is a SET or GET req
        let low32 = p.randomness & 0xffffffff;
        let key = (p.randomness >> 32) % NVALUES as u64;

        if low32 % 1000 < PCT_SET {
            MemcachedProtocol::usr_set_request(key, i as u32, buf, tport);
            return;
        }

        if let Transport::Udp = tport {
            buf.extend_from_slice(UDP_HEADER);
        }

        PacketHeader {
            magic: Magic::Request as u8,
            opcode: Opcode::Get as u8,
            key_length: KEY_SIZE as u16,
            total_body_length: KEY_SIZE as u32,
            opaque: i as u32,
            ..Default::default()
        }
        .write(buf)
        .unwrap();

        write_key(buf, key, KEY_SIZE);
    }

    pub fn etc_value_size(rng: &mut ThreadRng) -> usize {
        let mut sum = 0.0;
        let rand = rng.gen::<f64>();
        for (p, size) in ETC_VALUE_DISTR1 {
            sum += p;
            if rand < sum {
                return size;
            }
        }
        ETC_VALUE_DISTR2.sample(rng) as usize
    }

    pub fn etc_set_request(key: u64, opaque: u32, buf: &mut Vec<u8>, tport: Transport) {
        if let Transport::Udp = tport {
            buf.extend_from_slice(UDP_HEADER);
        }
        let mut rng = rand::thread_rng();
        let value_size = MemcachedProtocol::etc_value_size(&mut rng);
        let key_size = unsafe {
            ETC_KEY_PRELOAD[key as usize % NVALUES] =
                usize::max(usize::min(ETC_KEY_DISTR.sample(&mut rng) as usize, 256), KEY_SIZE);
            ETC_KEY_PRELOAD[key as usize % NVALUES]
        };
        println!("set {} {} {}", key, key_size, value_size);

        PacketHeader {
            magic: Magic::Request as u8,
            opcode: Opcode::Set as u8,
            key_length: key_size as u16,
            extras_length: 8,
            total_body_length: (8 + key_size + value_size) as u32,
            opaque,
            ..Default::default()
        }
        .write(buf)
        .unwrap();

        buf.write_u64::<BigEndian>(0).unwrap();

        write_key(buf, key, key_size as usize);

        for i in 0..value_size {
            buf.push((((key * i as u64) >> (i % 4)) & 0xff) as u8);
        }
    }

    pub fn gen_etc_request(i: usize, p: &Packet, buf: &mut Vec<u8>, tport: Transport) {
        // Use first 32 bits of randomness to determine if this is a SET or GET req
        let low32 = p.randomness & 0xffffffff;
        let key = (p.randomness >> 32) % NVALUES as u64;

        if low32 % 1000 < ETC_PCT_SET {
            MemcachedProtocol::etc_set_request(key, i as u32, buf, tport);
            return;
        }

        if let Transport::Udp = tport {
            buf.extend_from_slice(UDP_HEADER);
        }

        let key_size = unsafe { ETC_KEY_PRELOAD[key as usize % NVALUES] } as u16;
        // println!("get {} {}", key, key_size);
        PacketHeader {
            magic: Magic::Request as u8,
            opcode: Opcode::Get as u8,
            key_length: key_size,
            total_body_length: key_size as u32,
            opaque: i as u32,
            ..Default::default()
        }
        .write(buf)
        .unwrap();

        write_key(buf, key, key_size as usize);
    }

    pub fn set_request(key: u64, opaque: u32, buf: &mut Vec<u8>, tport: Transport) {
        // MemcachedProtocol::etc_set_request(key, opaque, buf, tport);
        MemcachedProtocol::usr_set_request(key, opaque, buf, tport);
    }

    pub fn gen_request(i: usize, p: &Packet, buf: &mut Vec<u8>, tport: Transport) {
        // MemcachedProtocol::gen_etc_request(i, p, buf, tport);
        MemcachedProtocol::gen_usr_request(i, p, buf, tport);
    }

    pub fn read_response(
        mut sock: &Connection,
        tport: Transport,
        scratch: &mut [u8],
    ) -> io::Result<usize> {
        let hdr = match tport {
            Transport::Udp => {
                let len = sock.read(&mut scratch[..32])?;
                if len == 0 {
                    return Err(Error::new(ErrorKind::UnexpectedEof, "eof"));
                }
                if len < 8 {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("Short packet received: {} bytes", len),
                    ));
                }
                PacketHeader::read(&mut &scratch[8..])?
            }
            Transport::Tcp => {
                sock.read_exact(&mut scratch[..24])?;
                let hdr = PacketHeader::read(&mut &scratch[..])?;
                if let Err(e) = sock.read_exact(&mut scratch[..hdr.total_body_length as usize]) {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("{} {}", e, hdr.total_body_length),
                    ));
                };
                hdr
            }
        };

        if hdr.vbucket_id_or_status != ResponseStatus::NoError as u16 {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Not NoError {}", hdr.vbucket_id_or_status),
            ));
        }
        Ok(hdr.opaque as usize)
    }
}
