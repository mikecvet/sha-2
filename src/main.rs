use clap::{arg, Command};
use std::{fs, io::Read};

const MAX_LEN:usize = 18446744073709551615;

// From https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf # 5.3.2
const SHA_224_H_INIT: [u32; 8] = [
    0xc1059ed8, 0x367cd507, 0x3070dd17, 0xf70e5939, 0xffc00b31, 0x68581511, 0x64f98fa7, 0xbefa4fa4
];

// From https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf # 5.3.3
const SHA_256_H_INIT: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
];

/*
 * From https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf # 4.2.2 
 *
 * SHA-224 and SHA-256 use the same sequence of sixty-four constant 32-bit words,
 * K{256}_0, K{256}_1, ..., K{256}_63. These words represent the first thirty-two bits of the 
 * fractional parts of the cube roots of the first sixty-four prime numbers.
 */
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
    h: u32,
    n: usize
}

impl State {
    fn new (n: usize) -> State {       

        // Select the appropriate initialization values based on algorithm 
        let init: &[u32; 8] = match n {
            224 => &SHA_224_H_INIT,
            256 => &SHA_256_H_INIT,
            _ => panic!("unsupported hash length"),
        };

        State {
            a: init[0],
            b: init[1],
            c: init[2],
            d: init[3],
            e: init[4],
            f: init[5],
            g: init[6],
            h: init[7],
            n: n
        }
    }

    /**
     * Rotates state values according to https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf 6.2.2 s
     * section 3.
     */
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

        if self.n == 256 {
            bytes.extend_from_slice(&self.h.to_be_bytes());
        }

        return bytes;
    }
}

/**
 * See https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf # 5.1.2
 * 
 * Suppose the length of the message M, in bits, is  bits. Append the bit “1” to the end of the message, 
 * followed by k zero bits, where k is the smallest non-negative solution to the equation L + 1 + k = 896 mod 1024. 
 * Then append the 128-bit block that is equal to the number L expressed using a binary representation. 
 * For example, the (8-bit ASCII) message “abc” has length 8 x 3 = 24, so the message is padded with a one bit,
 * then 896 - (24 + 1) = 871 zero bits, and then the message length, to become the 1024-bit padded message.
 * The length of the padded message should now be a multiple of 1024 bits.
 */
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

/**
 * Convenience function for passing strings; converts given string to a Vector of u8 bytes for 
 * the hash() function.
 */
fn hash_string (message: &str, n: usize) -> String {
    let mut message_bytes = message.as_bytes().to_vec();
    return hash (&mut message_bytes, n);
}

fn
hash (message: &mut Vec<u8>, n: usize) -> String {

    let mut state:State = State::new(n);

    // Extend to a multiple of 512 bits
    pad (message);

    /*
    * From https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf # 5.2
    * 
    * For SHA-1, SHA-224 and SHA-256, the message and its padding are parsed into N 512-bit blocks, M(1), M(2),..., M(N). 
    * Since the 512 bits of the input block may be expressed as sixteen 32-bit words, the first 32 bits of message 
    * block i are denoted M0(i), the next 32 bits are M1(i), and so on up to M(i).
    * 
    * For SHA-384, SHA-512, SHA-512/224 and SHA-512/256, the message and its padding are parsed into N 1024-bit blocks, 
    * M(1), M(2),..., M(N). Since the 1024 bits of the input block may be expressed as sixteen 64-bit words, the first 
    * 64 bits of message block i are denoted M0(i), the next 64 bits are M(i), and so on up to M(i).
    */
    for outer_block in message.chunks(64) {
        let mut w: [u32; 64] = [0; 64];
        let mut indx = 0;

        // Fill first 16 elements of w array with 32-bit integer from the 512-bit block
        // See https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf # 6.2.2
        for chunk in outer_block.chunks(4) {
            // Convert message byte chunks into a big-endian u32 integer and insert into w[indx]
            let (b1, b2, b3, b4) = (chunk[0] as u32, chunk[1] as u32, chunk[2] as u32, chunk[3] as u32);
            w[indx] = (b1 << 24) | (b2 << 16) | (b3 << 8) | b4;
            indx += 1;
        }

        // 16 .. 63
        while indx < 64 {
            /* 
            * From https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf # 4.1.2
            *
            * The two functions σ0 and σ1 as defined in the specification.
            */
            let s0 = (w[indx - 15].rotate_right(7)) 
                        ^ (w[indx - 15].rotate_right(18)) 
                        ^ (w[indx - 15] >> 3);
            let s1 = (w[indx - 2].rotate_right(17)) 
                        ^ (w[indx - 2].rotate_right(19)) 
                        ^ (w[indx - 2] >> 10);

            // From https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf # 6.2.2            
            w[indx] = w[indx - 16]
                        .wrapping_add(s0)
                        .wrapping_add(w[indx - 7])
                        .wrapping_add(s1);
            indx += 1;
        }

        // Stored to add back to the state after the main processing loop
        let input_values: [u32; 8] = [state.a, state.b, state.c, state.d, state.e, state.f, state.g, state.h];
        indx = 0;

        // See https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf # 6.2.2
        while indx < 64 {
            /* 
            * From https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf # 4.1.2
            *
            * The functions Σ0, Σ1, Ch(x, y, z) and Maj(x, y, z)
            */
            let s0 = state.a.rotate_right(2) ^ state.a.rotate_right(13) ^ state.a.rotate_right(22);
            let s1 = state.e.rotate_right(6) ^ state.e.rotate_right(11) ^ state.e.rotate_right(25);

            let ch = (state.e & state.f) ^ ((!state.e) & state.g);
            let maj = (state.a & state.b) ^ (state.a & state.c) ^ (state.b & state.c);

            // See https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf # 6.2.2 section 3
            state.rotate(
                state.h.wrapping_add(s1)
                  .wrapping_add(ch)
                  .wrapping_add(K[indx])
                  .wrapping_add(w[indx]),
                s0.wrapping_add(maj)
            );

            indx += 1;
        }

        state.add(&input_values);
    }

    // Encode state into base 64
    return hex::encode(
        &state.export()
    ); 
}

fn 
tests () {
    assert!(hash_string("", 256).eq("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"));
    assert!(hash_string("abcde", 256).eq("36bbe50ed96841d10443bcb670d6554f0a34b761be67ec9c4a8ad2c0c44ca42c"));
    assert!(hash_string("abcdefghijklmnopqrstuvwxyz12345678901234567890", 256)
      .eq("a8143361b55756a30c4c4369726748e4ae193ca1d31e1f21f47bc7171cd56e9a"));
    assert!(hash_string("a8143361b55756a30c4c4369726748e4ae193ca1d31e1f21f47bc7171cd56e9a", 256)
      .eq("fc3b517b3c9ede5c64058615d49ec4ac6eadda73d74f1eade0bdb5d70de93dfb"));

    assert!(hash_string("", 224).eq("d14a028c2a3a2bc9476102bb288234c415a2b01f828ea62ac5b3e42f"));
    assert!(hash_string("abcde", 224).eq("bdd03d560993e675516ba5a50638b6531ac2ac3d5847c61916cfced6"));
    assert!(hash_string("abcdefghijklmnopqrstuvwxyz12345678901234567890", 224)
        .eq("bbf04b42f9aa379d73e39955828523db73f5ddef6f8ca518684fb2b7"));
    assert!(hash_string("bbf04b42f9aa379d73e39955828523db73f5ddef6f8ca518684fb2b7", 224)
    .eq("e8cffc71ed2e47380e3ae16a92a6f5cfeb1f393a59f05d2cd05d72af"));

    println!("Tests completed successfully!");
}

fn 
main () {
    let matches = Command::new("sha2")
    .version("0.1")
    .about("Fun with cryptographic hash functions")
    .arg(arg!(--path <VALUE>).required(false))
    .arg(arg!(--string <VALUE>).required(false))
    .arg(arg!(--algo <VALUE>).required(false))
    .arg(arg!(--test).required(false))
    .get_matches();

    let string = matches.get_one::<String>("string");
    let path = matches.get_one::<String>("path");
    let algo = matches.get_one::<String>("algo");
    let test = matches.get_one::<bool>("test");

    let n = match algo.as_deref() {
        None => {
            if test.is_none() {
                println!("no algorithim specified; assuming SHA-256");
            }
            256
        },
        Some(s) => match s.as_str() {
            "224" => 224,
            "256" => 256,
            _ => panic!("unsupported algorithm; provide either '224' or '256'"),
        },
    };

    match (string, path, test) {
        (Some(&ref text), None, Some(false)) => {
            let digest = hash_string(&text, n);
            println!("{}", digest);
        },
        (None, Some(f), Some(false)) => {
            let mut file_data: Vec<u8> = Vec::new();
            let mut file = fs::File::open(f).expect("unable to open file");

            file.read_to_end(&mut file_data).expect("unable to read data");

            let digest = hash(&mut file_data, n);
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