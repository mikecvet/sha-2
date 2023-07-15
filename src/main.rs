use clap::{arg, Command};
use std::fs;

const MAX_LEN:usize = 18446744073709551615;

const H0: u32 = 0x6a09e667;
const H1: u32 = 0xbb67ae85;
const H2: u32 = 0x3c6ef372;
const H3: u32 = 0xa54ff53a;
const H4: u32 = 0x510e527f;
const H5: u32 = 0x9b05688c;
const H6: u32 = 0x1f83d9ab;
const H7: u32 = 0x5be0cd19;

const K: [u32; 64] = [
   0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
   0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
   0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
   0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
   0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
   0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
   0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
   0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
];

/*
 * Four 32-bit integer maintaining the state of the digest during hashing.
 */
struct State {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    e: u32,
    f: u32,
    g: u32,
    h: u32
}

impl Default for State {
    /**
     * Default constructor; initializes each of the State fields 
     * to the initial values from 3.3
     */
    fn default () -> State {
        State {
            a: H0,
            b: H1,
            c: H2,
            d: H3,
            e: H4,
            f: H5,
            g: H6,
            h: H7
        }
    }
}

impl State {
    fn rotate (&mut self, x: u32, y: u32) {
       self.h = self.g;
       self.g = self.f;
       self.f = self.e;
       self.e = self.d.wrapping_add(x);
       self.d = self.c;
       self.c = self.b;
       self.b = self.a;
       self.a = x.wrapping_add(y);
    }

    fn add (&mut self, v: &[u32; 8]) {
        self.a = self.a.wrapping_add(v[0]);
        self.b = self.b.wrapping_add(v[1]);
        self.c = self.c.wrapping_add(v[2]);
        self.d = self.d.wrapping_add(v[3]);
        self.e = self.e.wrapping_add(v[4]);
        self.f = self.f.wrapping_add(v[5]);
        self.g = self.g.wrapping_add(v[6]);
        self.h = self.h.wrapping_add(v[7]);
    }

    /**
     * Returns a byte vector representation of this State's integers
     */
    fn export (&mut self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.a.to_be_bytes());
        bytes.extend_from_slice(&self.b.to_be_bytes());
        bytes.extend_from_slice(&self.c.to_be_bytes());
        bytes.extend_from_slice(&self.d.to_be_bytes());
        bytes.extend_from_slice(&self.e.to_be_bytes());
        bytes.extend_from_slice(&self.f.to_be_bytes());
        bytes.extend_from_slice(&self.g.to_be_bytes());
        bytes.extend_from_slice(&self.h.to_be_bytes());

        return bytes;
    }
}

fn
pad (message: &mut Vec<u8>) {
    let mlen_in_bits = message.len() * 8 % MAX_LEN;

    // Appends 1 << 7, ie 1000 0000, we're working in bytes
    message.push(0x80);

    // Padding to 448 modulo 512 bits
    while (message.len() * 8 % MAX_LEN) % 512 != 448 {
        message.push(0x0);
    }

    let len_in_bytes = mlen_in_bits.to_be_bytes();
    message.extend_from_slice(&len_in_bytes);
}

fn
hash (message: &str) -> String {

    let mut state:State = Default::default();
    let mut message_bytes = message.as_bytes().to_vec();

    pad (&mut message_bytes);

    let b64_string = base64::encode(&message_bytes);
    println!("input message: {}", b64_string);

    let s = message_bytes.iter().map(|byte| byte.to_string()).collect::<Vec<String>>().join(" ");
    println!("input message: {}", s);

    // 512-bit chunks
    for outer_block in message_bytes.chunks(64) {
        let mut w: [u32; 64] = [0; 64];
        let mut indx = 0;

        // Fill M array with 32-bit integer from the outer 512-bit chunk
        for chunk in outer_block.chunks(4) {
            // Convert bytes into little-endian u32 integer and insert into m[indx]
            let (b1, b2, b3, b4) = (chunk[0] as u32, chunk[1] as u32, chunk[2] as u32, chunk[3] as u32);
            // w[indx] = (b4 << 24) | (b3 << 16) | (b2 << 8) | b1;
            w[indx] = b4 | (b3 << 8) | (b2 << 16) | (b1 << 24);
            indx += 1;
        }

        // 16 .. 63
        while indx < 64 {
            let s0 = (w[indx - 15].rotate_right(7)) ^ (w[indx - 15].rotate_right(18)) ^ (w[indx - 15] >> 3);
            let s1 = (w[indx - 2].rotate_right(17)) ^ (w[indx - 2].rotate_right(19)) ^ (w[indx - 2] >> 10);

            w[indx] = w[indx - 16].wrapping_add(s0).wrapping_add(w[indx - 7]).wrapping_add(s1);
            indx += 1;
        }

        let input_values: [u32; 8] = [state.a, state.b, state.c, state.d, state.e, state.f, state.g, state.h];

        indx = 0;
        while indx < 64 {
            let s1 = state.e.rotate_right(6) ^ state.e.rotate_right(11) ^ state.e.rotate_right(25);
            let ch = (state.e & state.f) ^ ((!state.e) & state.g);
            let temp1 = state.h.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[indx]).wrapping_add(w[indx]);
            let s0 = state.a.rotate_right(2) ^ state.a.rotate_right(13) ^ state.a.rotate_right(22);
            let maj = (state.a & state.b) ^ (state.a & state.c) ^ (state.b & state.c);
            let temp2 = s0.wrapping_add(maj);

            state.rotate(temp1, temp2);

            indx += 1;
        }

        state.add(&input_values);
    }

    // Export state integers into a byte array
    let digest: [u8; 32] = state.export().try_into().expect("Wrong length");

    let s = digest.iter().map(|byte| byte.to_string()).collect::<Vec<String>>().join(" ");
    println!("output digest: {}", s);

    // Encode into base 64
    return hex::encode(&digest); 
}

fn 
tests () {
    assert!(hash("").eq("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"));
    assert!(hash("abcde").eq("36bbe50ed96841d10443bcb670d6554f0a34b761be67ec9c4a8ad2c0c44ca42c"));
    assert!(hash("abcdefghijklmnopqrstuvwxyz12345678901234567890")
      .eq("a8143361b55756a30c4c4369726748e4ae193ca1d31e1f21f47bc7171cd56e9a"));
}

fn 
main () {
    let matches = Command::new("sha2")
    .version("0.1")
    .about("Fun with cryptographic hash functions")
    .arg(arg!(--path <VALUE>).required(false))
    .arg(arg!(--string <VALUE>).required(false))
    .arg(arg!(--test).required(false))
    .get_matches();

    let string = matches.get_one::<String>("string");
    let path = matches.get_one::<String>("path");
    let test = matches.get_one::<bool>("test");

    match (string, path, test) {
        (Some(text), None, Some(false)) => {
            let digest = hash(&text);
            println!("{}", digest);
        },
        (None, Some(f), Some(false)) => {
            let contents = fs::read_to_string(f)
                .expect("Should have been able to read the file");
            let digest = hash(&contents);
            println!("{}", digest);
        },
        (None, None, Some(true)) => {
            tests();
        }
        _ => {
            println!("no text provided!");
        }
    }
}