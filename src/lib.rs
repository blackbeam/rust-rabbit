#![cfg_attr(feature = "nostd", feature(no_std, core_slice_ext))]
#![cfg_attr(feature = "nostd", no_std)]

#[cfg(not(feature = "nostd"))]
use std::io;
#[cfg(not(feature = "nostd"))]
use std::fmt;
#[cfg(not(feature = "nostd"))]
use std::ops::Deref;

#[cfg(feature = "nostd")]
use core::ops::Deref;

const WORDSIZE: u64 = (1 << 32);
const A: [u32; 8] = [0x4D34D34D, 0xD34D34D3, 0x34D34D34, 0x4D34D34D,
                     0xD34D34D3, 0x34D34D34, 0x4D34D34D, 0xD34D34D3];

/// 128-bit key
///
/// ```ignore
/// ...
/// let key1: Key = [0u8; 16].into();
/// let key2: Key = byte_slice_with_len_eq_16.into();
/// let key3: Key = byte_slice_with_len_ne_16.into(); // Panic!
/// ```
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Key([u8; 16]);

impl Deref for Key {
    type Target = [u8; 16];
    fn deref(&self) -> &[u8; 16] {
        &self.0
    }
}

impl From<[u8; 16]> for Key {
    fn from(key: [u8; 16]) -> Key {
        Key(key)
    }
}

/// Asserts that slice.len() == 16
impl<'a> From<&'a [u8]> for Key {
    fn from(slice: &[u8]) -> Key {
        assert_eq!(slice.len(), 16);
        let mut key = [0; 16];
        for i in 0..16 {
            key[i] = slice[i];
        }
        Key(key)
    }
}

/// 64-bit initialization vector
///
/// ```ignore
/// let iv1: InitVec = u64.into(); // LSB -> iv[0], MSB -> iv[7]
/// let iv2: InitVec = [0u8; 8].into();
/// let iv3: InitVec = byte_slice_with_len_eq_8.into();
/// let iv4: InitVec = byte_slice_with_len_ne_8.into(); // Panic!
/// ```
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct InitVec([u8; 8]);

impl Deref for InitVec {
    type Target = [u8; 8];
    fn deref(&self) -> &[u8; 8] {
        &self.0
    }
}

impl From<u64> for InitVec {
    fn from(v: u64) -> InitVec {
        let mut iv = [0u8; 8];
        for i in 0..8 {
            iv[i] = (v >> (i * 8)) as u8;
        }
        InitVec(iv)
    }
}

impl From<[u8; 8]> for InitVec {
    fn from(iv: [u8; 8]) -> InitVec {
        InitVec(iv)
    }
}

impl<'a> From<&'a [u8]> for InitVec {
    fn from(slice: &'a [u8]) -> InitVec {
        assert_eq!(slice.len(), 8);
        let mut iv = [0; 8];
        for i in 0..8 {
            iv[i] = slice[i];
        }
        InitVec(iv)
    }
}

#[derive(Default, Clone, PartialEq, Eq, Hash, Debug)]
struct State {
    state_vars: [u32; 8],
    counter_vars: [u32; 8],
    carry_bit: u8,
}

#[cfg(not(feature = "nostd"))]
impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.carry_bit > 0 {
            try!(write!(f, "Carry bit: SET\n"));
        } else {
            try!(write!(f, "Carry bit: UNSET\n"));
        }
        for i in 0..8 {
            try!(write!(f, "X{} = 0x{:08X}, ", i, self.state_vars[i]));
            if i == 3 || i == 7 {
                try!(write!(f, "\n"));
            }
        }
        for i in 0..8 {
            try!(write!(f, "C{} = 0x{:08X}, ", i, self.counter_vars[i]));
            if i == 3 || i == 7 {
                try!(write!(f, "\n"));
            }
        }
        return Ok(());
    }
}

fn setup_key(state: &mut State, key: &Key) {
    let mut k = [0u16; 8];
    k[0] = (key[0x0] as u16) | ((key[0x1] as u16) << 8);
    k[1] = (key[0x2] as u16) | ((key[0x3] as u16) << 8);
    k[2] = (key[0x4] as u16) | ((key[0x5] as u16) << 8);
    k[3] = (key[0x6] as u16) | ((key[0x7] as u16) << 8);
    k[4] = (key[0x8] as u16) | ((key[0x9] as u16) << 8);
    k[5] = (key[0xA] as u16) | ((key[0xB] as u16) << 8);
    k[6] = (key[0xC] as u16) | ((key[0xD] as u16) << 8);
    k[7] = (key[0xE] as u16) | ((key[0xF] as u16) << 8);

    for j in 0..8 {
        if j % 2 == 0 {
            state.state_vars[j] = ((k[(j + 1) % 8] as u32) << 16) | (k[j] as u32);
            state.counter_vars[j] = ((k[(j + 4) % 8] as u32) << 16) | (k[(j + 5) % 8] as u32);
        } else {
            state.state_vars[j] = ((k[(j + 5) % 8] as u32) << 16) | (k[(j + 4) % 8] as u32);
            state.counter_vars[j] = ((k[j] as u32) << 16) | (k[(j + 1) % 8] as u32);
        }
    }


    for _ in 0..4 {
        next_state(state);
    }

    for j in 0..8 {
        state.counter_vars[j] = state.counter_vars[j] ^ state.state_vars[(j + 4) % 8];
    }
}

fn setup_iv(state: &mut State, iv: &InitVec) {
    let i0 = iv[0] as u32 | (iv[1] as u32) << 8 | (iv[2] as u32) << 16 | (iv[3] as u32) << 24;
    let i2 = iv[4] as u32 | (iv[5] as u32) << 8 | (iv[6] as u32) << 16 | (iv[7] as u32) << 24;
    let i1 = (i0 >> 16) | (i2 & 0xFFFF0000);
    let i3 = (i2 << 16) | (i0 & 0x0000FFFF);

    state.counter_vars[0] = state.counter_vars[0] ^ i0;
    state.counter_vars[1] = state.counter_vars[1] ^ i1;
    state.counter_vars[2] = state.counter_vars[2] ^ i2;
    state.counter_vars[3] = state.counter_vars[3] ^ i3;
    state.counter_vars[4] = state.counter_vars[4] ^ i0;
    state.counter_vars[5] = state.counter_vars[5] ^ i1;
    state.counter_vars[6] = state.counter_vars[6] ^ i2;
    state.counter_vars[7] = state.counter_vars[7] ^ i3;

    for _ in 0..4 {
        next_state(state);
    }
}

fn counter_update(state: &mut State) {
    for j in 0..8 {
        let temp = state.counter_vars[j] as u64 + A[j] as u64 + state.carry_bit as u64;
        state.carry_bit = ((temp / WORDSIZE) as u8) & 0b1;
        state.counter_vars[j] = (temp % WORDSIZE) as u32;
    }
}

fn next_state(state: &mut State) {
    let mut g = [0u32; 8];

    counter_update(state);

    for j in 0..8 {
        let u_plus_v = state.state_vars[j] as u64 + state.counter_vars[j] as u64;
        let square_uv = (u_plus_v % WORDSIZE) * (u_plus_v % WORDSIZE);
        g[j] = (square_uv ^ (square_uv >> 32)) as u32;
    }

    state.state_vars[0] = g[0].wrapping_add(g[7].rotate_left(16))
                              .wrapping_add(g[6].rotate_left(16));
    state.state_vars[1] = g[1].wrapping_add(g[0].rotate_left(8)).wrapping_add(g[7]);
    state.state_vars[2] = g[2].wrapping_add(g[1].rotate_left(16))
                              .wrapping_add(g[0].rotate_left(16));
    state.state_vars[3] = g[3].wrapping_add(g[2].rotate_left(8)).wrapping_add(g[1]);
    state.state_vars[4] = g[4].wrapping_add(g[3].rotate_left(16))
                              .wrapping_add(g[2].rotate_left(16));
    state.state_vars[5] = g[5].wrapping_add(g[4].rotate_left(8)).wrapping_add(g[3]);
    state.state_vars[6] = g[6].wrapping_add(g[5].rotate_left(16))
                              .wrapping_add(g[4].rotate_left(16));
    state.state_vars[7] = g[7].wrapping_add(g[6].rotate_left(8)).wrapping_add(g[5]);
}

fn extract(state: &State) -> [u8; 16] {
    let mut s = [0u8; 16];

    let s15_0 = ((state.state_vars[0]) ^ (state.state_vars[5] >> 16)) as u16;
    let s31_16 = ((state.state_vars[0] >> 16) ^ (state.state_vars[3])) as u16;
    let s47_32 = ((state.state_vars[2]) ^ (state.state_vars[7] >> 16)) as u16;
    let s63_48 = ((state.state_vars[2] >> 16) ^ (state.state_vars[5])) as u16;
    let s79_64 = ((state.state_vars[4]) ^ (state.state_vars[1] >> 16)) as u16;
    let s95_80 = ((state.state_vars[4] >> 16) ^ (state.state_vars[7])) as u16;
    let s111_96 = ((state.state_vars[6]) ^ (state.state_vars[3] >> 16)) as u16;
    let s127_112 = ((state.state_vars[6] >> 16) ^ (state.state_vars[1])) as u16;

    s[0x0] = s15_0 as u8;
    s[0x1] = (s15_0 >> 8) as u8;
    s[0x2] = s31_16 as u8;
    s[0x3] = (s31_16 >> 8) as u8;
    s[0x4] = s47_32 as u8;
    s[0x5] = (s47_32 >> 8) as u8;
    s[0x6] = s63_48 as u8;
    s[0x7] = (s63_48 >> 8) as u8;
    s[0x8] = s79_64 as u8;
    s[0x9] = (s79_64 >> 8) as u8;
    s[0xA] = s95_80 as u8;
    s[0xB] = (s95_80 >> 8) as u8;
    s[0xC] = s111_96 as u8;
    s[0xD] = (s111_96 >> 8) as u8;
    s[0xE] = s127_112 as u8;
    s[0xF] = (s127_112 >> 8) as u8;

    return s;
}

pub struct Rabbit {
    master_state: State,
    state: State,
    buf: [u8; 16],
    buf_idx: u8,
}

impl Rabbit {
    /// Setupы given `key` on an empty rabbit state.
    pub fn new(key: &Key) -> Rabbit {
        let mut state = Default::default();
        setup_key(&mut state, key);
        Rabbit {
            master_state: state.clone(),
            state: state,
            buf: [0; 16],
            buf_idx: 0x10,
        }
    }

    /// Setupы given `key` on an empty rabbit state, then setupы initialization vector `iv` on it.
    pub fn new_iv(key: &Key, iv: &InitVec) -> Rabbit {
        let mut state = Default::default();
        setup_key(&mut state, key);
        let master_state = state.clone();
        setup_iv(&mut state, iv);
        Rabbit {
            master_state: master_state,
            state: state,
            buf: [0; 16],
            buf_idx: 0x10,
        }
    }

    /// Restores master state.
    pub fn reset(&mut self) {
        self.state = self.master_state.clone();
        self.buf_idx = 0x10;
    }

    /// Restores master state, than setups initialization vector `iv` on it.
    pub fn reinit(&mut self, iv: &InitVec) {
        self.state = self.master_state.clone();
        self.buf_idx = 0x10;
        setup_iv(&mut self.state, iv);
    }

    /// Encrypts and writes bytes of `data` to `buf`.
    /// Asserts that `buf.len() >= data.len()`.
    pub fn encrypt(&mut self, data: &[u8], buf: &mut [u8]) {
        assert!(buf.len() >= data.len());
        for i in 0..data.len() {
            buf[i] = data[i] ^ self.get_s_byte();
        }
    }

    #[inline]
    /// Decrypts and writes bytes of `data` to `buf`.
    /// Asserts that `buf.len() >= data.len()`.
    pub fn decrypt(&mut self, data: &[u8], buf: &mut [u8]) {
        self.encrypt(data, buf)
    }

    /// Encrypts bytes of `data` inplace.
    pub fn encrypt_inplace(&mut self, data: &mut [u8]) {
        for i in 0..data.len() {
            data[i] ^= self.get_s_byte();
        }
    }

    #[inline]
    /// Decrypts bytes of `data` inplace.
    pub fn decrypt_inplace(&mut self, data: &mut [u8]) {
        self.encrypt_inplace(data)
    }

    #[inline]
    fn get_s_byte(&mut self) -> u8 {
        if self.buf_idx == 0x10 {
            next_state(&mut self.state);
            let s = extract(&mut self.state);
            self.buf = s;
            self.buf_idx = 0;
        }
        let byte = self.buf[self.buf_idx as usize];
        self.buf_idx += 1;
        byte
    }
}

#[cfg(not(feature = "nostd"))]
/// Wrapper for `io::Read` and `io::Write` implementors.
pub struct Stream<S> {
    stream: S,
    rabbit: Rabbit,
}

#[cfg(not(feature = "nostd"))]
impl<S> Stream<S> {
    pub fn new(rabbit: Rabbit, stream: S) -> Stream<S> {
        Stream {
            stream: stream,
            rabbit: rabbit,
        }
    }

    pub fn into_inner(self) -> (Rabbit, S) {
        let rabbit = self.rabbit;
        let stream = self.stream;
        (rabbit, stream)
    }
}

#[cfg(not(feature = "nostd"))]
impl<S: io::Read> io::Read for Stream<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = try!(self.stream.read(buf));
        self.rabbit.encrypt_inplace(&mut buf[0..count]);
        Ok(count)
    }
}

#[cfg(not(feature = "nostd"))]
impl<S: io::Write> io::Write for Stream<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut vec = vec![0; buf.len()];
        self.rabbit.encrypt(buf, &mut vec[..]);
        try!(self.stream.write_all(&vec[..]));
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

#[cfg(test)]
mod test {
    #[cfg(not(feature = "nostd"))]
    use std::io::Read;
    #[cfg(not(feature = "nostd"))]
    use std::io::Write;

    use super::{
        Key,
        InitVec,
        Rabbit,
        extract,
        next_state,
        setup_key,
        setup_iv,
    };

    #[cfg(not(feature = "nostd"))]
    use super::Stream;

    macro_rules! test_raw {
        ($name:ident $wrap_name:ident $stream_name:ident
         key  = [$kf:expr, $ke:expr, $kd:expr, $kc:expr,
                 $kb:expr, $ka:expr, $k9:expr, $k8:expr,
                 $k7:expr, $k6:expr, $k5:expr, $k4:expr,
                 $k3:expr, $k2:expr, $k1:expr, $k0:expr]
         S[0] = [$s0f:expr, $s0e:expr, $s0d:expr, $s0c:expr,
                 $s0b:expr, $s0a:expr, $s09:expr, $s08:expr,
                 $s07:expr, $s06:expr, $s05:expr, $s04:expr,
                 $s03:expr, $s02:expr, $s01:expr, $s00:expr]
         S[1] = [$s1f:expr, $s1e:expr, $s1d:expr, $s1c:expr,
                 $s1b:expr, $s1a:expr, $s19:expr, $s18:expr,
                 $s17:expr, $s16:expr, $s15:expr, $s14:expr,
                 $s13:expr, $s12:expr, $s11:expr, $s10:expr]
         S[2] = [$s2f:expr, $s2e:expr, $s2d:expr, $s2c:expr,
                 $s2b:expr, $s2a:expr, $s29:expr, $s28:expr,
                 $s27:expr, $s26:expr, $s25:expr, $s24:expr,
                 $s23:expr, $s22:expr, $s21:expr, $s20:expr]) =>
        {
            #[test]
            fn $name() {
                let key = Key([$k0,$k1,$k2,$k3,$k4,$k5,$k6,$k7,$k8,$k9,$ka,$kb,$kc,$kd,$ke,$kf]);
                let s0 = [$s00,$s01,$s02,$s03,$s04,$s05,$s06,$s07,
                          $s08,$s09,$s0a,$s0b,$s0c,$s0d,$s0e,$s0f];
                let s1 = [$s10,$s11,$s12,$s13,$s14,$s15,$s16,$s17,
                          $s18,$s19,$s1a,$s1b,$s1c,$s1d,$s1e,$s1f];
                let s2 = [$s20,$s21,$s22,$s23,$s24,$s25,$s26,$s27,
                          $s28,$s29,$s2a,$s2b,$s2c,$s2d,$s2e,$s2f];
                let mut state = Default::default();
                setup_key(&mut state, &key);
                next_state(&mut state);
                assert_eq!(extract(&state), s0);
                next_state(&mut state);
                assert_eq!(extract(&state), s1);
                next_state(&mut state);
                assert_eq!(extract(&state), s2);
            }

            #[test]
            fn $wrap_name() {
                let key = Key([$k0,$k1,$k2,$k3,$k4,$k5,$k6,$k7,$k8,$k9,$ka,$kb,$kc,$kd,$ke,$kf]);
                let s = [$s00,$s01,$s02,$s03,$s04,$s05,$s06,$s07,
                         $s08,$s09,$s0a,$s0b,$s0c,$s0d,$s0e,$s0f,
                         $s10,$s11,$s12,$s13,$s14,$s15,$s16,$s17,
                         $s18,$s19,$s1a,$s1b,$s1c,$s1d,$s1e,$s1f,
                         $s20,$s21,$s22,$s23,$s24,$s25,$s26,$s27,
                         $s28,$s29,$s2a,$s2b,$s2c,$s2d,$s2e,$s2f];
                let mut d = [0; 48];
                let mut rabbit = Rabbit::new(&key);
                rabbit.encrypt_inplace(&mut d);
                assert_eq!(&s[..], &d[..]);
                rabbit.reset();
                rabbit.encrypt(&[0; 48], &mut d);
                assert_eq!(&s[..], &d[..]);
            }

            #[cfg(not(feature = "nostd"))]
            #[test]
            fn $stream_name() {
                let key = Key([$k0,$k1,$k2,$k3,$k4,$k5,$k6,$k7,$k8,$k9,$ka,$kb,$kc,$kd,$ke,$kf]);
                let s = [$s00,$s01,$s02,$s03,$s04,$s05,$s06,$s07,
                         $s08,$s09,$s0a,$s0b,$s0c,$s0d,$s0e,$s0f,
                         $s10,$s11,$s12,$s13,$s14,$s15,$s16,$s17,
                         $s18,$s19,$s1a,$s1b,$s1c,$s1d,$s1e,$s1f,
                         $s20,$s21,$s22,$s23,$s24,$s25,$s26,$s27,
                         $s28,$s29,$s2a,$s2b,$s2c,$s2d,$s2e,$s2f];
                let mut d = [0; 48];
                let mut rabbit = Rabbit::new(&key);
                {
                    let mut stream = Stream::new(rabbit, &mut d[..]);
                    assert_eq!(48, stream.write(&[0; 48][..]).unwrap());
                    let (r, _) = stream.into_inner();
                    rabbit = r;
                }
                assert_eq!(&s[..], &d[..]);
                d = [0; 48];
                rabbit.reset();
                {
                    let tmp = [0; 48];
                    let mut stream = Stream::new(rabbit, &tmp[..]);
                    assert_eq!(48, stream.read(&mut d[..]).unwrap());
                }
                assert_eq!(&s[..], &d[..]);
            }
        };
        ($name:ident $wrap_name:ident $stream_name:ident
         key  = [$kf:expr, $ke:expr, $kd:expr, $kc:expr,
                 $kb:expr, $ka:expr, $k9:expr, $k8:expr,
                 $k7:expr, $k6:expr, $k5:expr, $k4:expr,
                 $k3:expr, $k2:expr, $k1:expr, $k0:expr]
         iv   = [$iv7:expr, $iv6:expr, $iv5:expr, $iv4:expr,
                 $iv3:expr, $iv2:expr, $iv1:expr, $iv0:expr]
         S[0] = [$s0f:expr, $s0e:expr, $s0d:expr, $s0c:expr,
                 $s0b:expr, $s0a:expr, $s09:expr, $s08:expr,
                 $s07:expr, $s06:expr, $s05:expr, $s04:expr,
                 $s03:expr, $s02:expr, $s01:expr, $s00:expr]
         S[1] = [$s1f:expr, $s1e:expr, $s1d:expr, $s1c:expr,
                 $s1b:expr, $s1a:expr, $s19:expr, $s18:expr,
                 $s17:expr, $s16:expr, $s15:expr, $s14:expr,
                 $s13:expr, $s12:expr, $s11:expr, $s10:expr]
         S[2] = [$s2f:expr, $s2e:expr, $s2d:expr, $s2c:expr,
                 $s2b:expr, $s2a:expr, $s29:expr, $s28:expr,
                 $s27:expr, $s26:expr, $s25:expr, $s24:expr,
                 $s23:expr, $s22:expr, $s21:expr, $s20:expr]) =>
        {
            #[test]
            fn $name() {
                let key = Key([$k0,$k1,$k2,$k3,$k4,$k5,$k6,$k7,$k8,$k9,$ka,$kb,$kc,$kd,$ke,$kf]);
                let iv = InitVec([$iv0, $iv1, $iv2, $iv3, $iv4, $iv5, $iv6, $iv7]);
                let s0 = [$s00,$s01,$s02,$s03,$s04,$s05,$s06,$s07,
                          $s08,$s09,$s0a,$s0b,$s0c,$s0d,$s0e,$s0f];
                let s1 = [$s10,$s11,$s12,$s13,$s14,$s15,$s16,$s17,
                          $s18,$s19,$s1a,$s1b,$s1c,$s1d,$s1e,$s1f];
                let s2 = [$s20,$s21,$s22,$s23,$s24,$s25,$s26,$s27,
                          $s28,$s29,$s2a,$s2b,$s2c,$s2d,$s2e,$s2f];
                let mut state = Default::default();
                setup_key(&mut state, &key);
                setup_iv(&mut state, &iv);
                next_state(&mut state);
                assert_eq!(extract(&state), s0);
                next_state(&mut state);
                assert_eq!(extract(&state), s1);
                next_state(&mut state);
                assert_eq!(extract(&state), s2);
            }

            #[test]
            fn $wrap_name() {
                let key = Key([$k0,$k1,$k2,$k3,$k4,$k5,$k6,$k7,$k8,$k9,$ka,$kb,$kc,$kd,$ke,$kf]);
                let iv = InitVec([$iv0, $iv1, $iv2, $iv3, $iv4, $iv5, $iv6, $iv7]);
                let s = [$s00,$s01,$s02,$s03,$s04,$s05,$s06,$s07,
                         $s08,$s09,$s0a,$s0b,$s0c,$s0d,$s0e,$s0f,
                         $s10,$s11,$s12,$s13,$s14,$s15,$s16,$s17,
                         $s18,$s19,$s1a,$s1b,$s1c,$s1d,$s1e,$s1f,
                         $s20,$s21,$s22,$s23,$s24,$s25,$s26,$s27,
                         $s28,$s29,$s2a,$s2b,$s2c,$s2d,$s2e,$s2f];
                let mut d = [0; 48];
                let mut rabbit = Rabbit::new_iv(&key, &iv);
                rabbit.encrypt_inplace(&mut d);
                assert_eq!(&s[..], &d[..]);
                rabbit.reinit(&iv);
                rabbit.encrypt(&[0; 48], &mut d);
                assert_eq!(&s[..], &d[..]);
            }

            #[cfg(not(feature = "nostd"))]
            #[test]
            fn $stream_name() {
                let key = Key([$k0,$k1,$k2,$k3,$k4,$k5,$k6,$k7,$k8,$k9,$ka,$kb,$kc,$kd,$ke,$kf]);
                let iv = InitVec([$iv0, $iv1, $iv2, $iv3, $iv4, $iv5, $iv6, $iv7]);
                let s = [$s00,$s01,$s02,$s03,$s04,$s05,$s06,$s07,
                         $s08,$s09,$s0a,$s0b,$s0c,$s0d,$s0e,$s0f,
                         $s10,$s11,$s12,$s13,$s14,$s15,$s16,$s17,
                         $s18,$s19,$s1a,$s1b,$s1c,$s1d,$s1e,$s1f,
                         $s20,$s21,$s22,$s23,$s24,$s25,$s26,$s27,
                         $s28,$s29,$s2a,$s2b,$s2c,$s2d,$s2e,$s2f];
                let mut d = [0; 48];
                let mut rabbit = Rabbit::new_iv(&key, &iv);
                {
                    let mut stream = Stream::new(rabbit, &mut d[..]);
                    assert_eq!(48, stream.write(&[0; 48][..]).unwrap());
                    let (r, _) = stream.into_inner();
                    rabbit = r;
                }
                assert_eq!(&s[..], &d[..]);
                d = [0; 48];
                rabbit.reinit(&iv);
                {
                    let tmp = [0; 48];
                    let mut stream = Stream::new(rabbit, &tmp[..]);
                    assert_eq!(48, stream.read(&mut d[..]).unwrap());
                }
                assert_eq!(&s[..], &d[..]);
            }
        }
    }

    // Without IV setup

    test_raw! {
        without_iv_setup1
        wrapped_without_iv1
        stream_without_iv1
        key  = [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00]
        S[0] = [0xB1,0x57,0x54,0xF0,0x36,0xA5,0xD6,0xEC,0xF5,0x6B,0x45,0x26,0x1C,0x4A,0xF7,0x02]
        S[1] = [0x88,0xE8,0xD8,0x15,0xC5,0x9C,0x0C,0x39,0x7B,0x69,0x6C,0x47,0x89,0xC6,0x8A,0xA7]
        S[2] = [0xF4,0x16,0xA1,0xC3,0x70,0x0C,0xD4,0x51,0xDA,0x68,0xD1,0x88,0x16,0x73,0xD6,0x96]
    }

    test_raw! {
        without_iv_setup2
        wrapped_without_iv2
        stream_without_iv2
        key  = [0x91,0x28,0x13,0x29,0x2E,0x3D,0x36,0xFE,0x3B,0xFC,0x62,0xF1,0xDC,0x51,0xC3,0xAC]
        S[0] = [0x3D,0x2D,0xF3,0xC8,0x3E,0xF6,0x27,0xA1,0xE9,0x7F,0xC3,0x84,0x87,0xE2,0x51,0x9C]
        S[1] = [0xF5,0x76,0xCD,0x61,0xF4,0x40,0x5B,0x88,0x96,0xBF,0x53,0xAA,0x85,0x54,0xFC,0x19]
        S[2] = [0xE5,0x54,0x74,0x73,0xFB,0xDB,0x43,0x50,0x8A,0xE5,0x3B,0x20,0x20,0x4D,0x4C,0x5E]
    }

    test_raw! {
        without_iv_setup3
        wrapped_without_iv3
        stream_without_iv3
        key  = [0x83,0x95,0x74,0x15,0x87,0xE0,0xC7,0x33,0xE9,0xE9,0xAB,0x01,0xC0,0x9B,0x00,0x43]
        S[0] = [0x0C,0xB1,0x0D,0xCD,0xA0,0x41,0xCD,0xAC,0x32,0xEB,0x5C,0xFD,0x02,0xD0,0x60,0x9B]
        S[1] = [0x95,0xFC,0x9F,0xCA,0x0F,0x17,0x01,0x5A,0x7B,0x70,0x92,0x11,0x4C,0xFF,0x3E,0xAD]
        S[2] = [0x96,0x49,0xE5,0xDE,0x8B,0xFC,0x7F,0x3F,0x92,0x41,0x47,0xAD,0x3A,0x94,0x74,0x28]
    }

    // With IV setup

    test_raw! {
        with_iv_setup1
        wrapped_with_iv1
        stream_with_iv1
        key  = [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00]
        iv   = [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00]
        S[0] = [0xC6,0xA7,0x27,0x5E,0xF8,0x54,0x95,0xD8,0x7C,0xCD,0x5D,0x37,0x67,0x05,0xB7,0xED]
        S[1] = [0x5F,0x29,0xA6,0xAC,0x04,0xF5,0xEF,0xD4,0x7B,0x8F,0x29,0x32,0x70,0xDC,0x4A,0x8D]
        S[2] = [0x2A,0xDE,0x82,0x2B,0x29,0xDE,0x6C,0x1E,0xE5,0x2B,0xDB,0x8A,0x47,0xBF,0x8F,0x66]
    }

    test_raw! {
        with_iv_setup2
        wrapped_with_iv2
        stream_with_iv2
        key  = [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00]
        iv   = [0xC3,0x73,0xF5,0x75,0xC1,0x26,0x7E,0x59]
        S[0] = [0x1F,0xCD,0x4E,0xB9,0x58,0x00,0x12,0xE2,0xE0,0xDC,0xCC,0x92,0x22,0x01,0x7D,0x6D]
        S[1] = [0xA7,0x5F,0x4E,0x10,0xD1,0x21,0x25,0x01,0x7B,0x24,0x99,0xFF,0xED,0x93,0x6F,0x2E]
        S[2] = [0xEB,0xC1,0x12,0xC3,0x93,0xE7,0x38,0x39,0x23,0x56,0xBD,0xD0,0x12,0x02,0x9B,0xA7]
    }

    test_raw! {
        with_iv_setup3
        wrapped_with_iv3
        stream_with_iv3
        key  = [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00]
        iv   = [0xA6,0xEB,0x56,0x1A,0xD2,0xF4,0x17,0x27]
        S[0] = [0x44,0x5A,0xD8,0xC8,0x05,0x85,0x8D,0xBF,0x70,0xB6,0xAF,0x23,0xA1,0x51,0x10,0x4D]
        S[1] = [0x96,0xC8,0xF2,0x79,0x47,0xF4,0x2C,0x5B,0xAE,0xAE,0x67,0xC6,0xAC,0xC3,0x5B,0x03]
        S[2] = [0x9F,0xCB,0xFC,0x89,0x5F,0xA7,0x1C,0x17,0x31,0x3D,0xF0,0x34,0xF0,0x15,0x51,0xCB]
    }
}
