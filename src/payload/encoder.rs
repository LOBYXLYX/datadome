use std::time::{SystemTime, UNIX_EPOCH};
pub struct PRNGEncoder {
    state: i64,
    counter: i64,
    byte_index: i32,
    cache: Option<i32>,
    global_flag: bool,
}

pub struct EncoderParsed {
    pub dynamic_num: i64,
    pub str_key: String
}

impl PRNGEncoder {
    fn new(seed: i64, counter_start: i64, global_flag: bool) -> Self {
        /* 
        let encoder_parsed: EncoderParsed = EncoderParsed {
            str_key: String::from("77DC0FFBAA0B77570F6B414F8E5BDB"),
            dynamic_num: -883841716
        };

        let input_key = String::from("u_zlE3FitM7uc5xiIspNTUPzm95L0AOHOJ7VbQ_orzjWo~32Zbz~HXVdqmAWMjn1woJzg1~TO_L8XWtPG~wYqHnm2S~_QzymmkIveIyEaILkNhY_kpjNULrEoqymUZMB");
        let mut enc_strings: Vec<String> = Vec::new();
        enc_strings.push("dwkqw".to_string());

        PRNGEncoder::encode(&encoder_parsed, enc_strings, input_key);
        */
        PRNGEncoder {
            state: seed,
            counter: counter_start,
            byte_index: -1,
            cache: None,
            global_flag,
        }
    }

    fn next(&mut self, set_cache: bool) -> i32 {
        if let Some(val) = self.cache.take() {
            return val;
        }

        self.byte_index += 1;
        if self.byte_index > 2 {
            self.state = PRNGEncoder::xorshift32(self.state);
            self.byte_index = 0;
        }

        let mut out: i32 = (self.state >> (16 - 8 * self.byte_index)) as i32;
        if self.global_flag {
            self.counter -= 1;
            out ^= self.counter as i32;
        }
        out &= 0xff;

        if set_cache {
            self.cache = Some(out);
        }

        out
    }

    fn string_hash(s: &str) -> i64 {
        let mut h: i64 = 0;
        for c in s.chars() {
            h = ((h << 5) - h + (c as i64)) as i64;
            h &= 0xffffffff;
        }
        h
    }

    fn xorshift32(mut num: i64) -> i64 {
        num ^= num << 13;
        num ^= num >> 17;
        num ^ (num << 5)
    }

    fn map_base64(val: i32) -> char {
        if val > 37 {
            (59 + val) as u8 as char
        } else if val > 11 {
            (53 + val) as u8 as char
        } else if val > 1 {
            (46 + val) as u8 as char
        } else {
            (50 * val + 45) as u8 as char
        }
    }

    fn custom_base64(bytes: &[i32], base_counter: &mut i64) -> String {
        let mut result: String = String::new();
        let mut i: usize = 0;

        while i < bytes.len() {
            *base_counter -= 1;
            let b1: i32 = (0xff & *base_counter as i32) ^ bytes[i];
            i += 1;

            *base_counter -= 1;
            let b2: i32 = if i < bytes.len() {
                let v: i32 = (0xff & *base_counter as i32) ^ bytes[i];
                i += 1;
                v
            } else {
                0
            };

            *base_counter -= 1;
            let b3 = if i < bytes.len() {
                let v = (0xff & *base_counter as i32) ^ bytes[i];
                i += 1;
                v
            } else {
                0
            };

            let group = (b1 << 16) | (b2 << 8) | b3;

            result.push(PRNGEncoder::map_base64((group >> 18) & 63));
            result.push(PRNGEncoder::map_base64((group >> 12) & 63));
            result.push(PRNGEncoder::map_base64((group >> 6) & 63));
            result.push(PRNGEncoder::map_base64(group & 63));
        }

        let padding: usize = bytes.len() % 3;
        if padding > 0 {
            let cut = 3 - padding;
            result.truncate(result.len() - cut);
        }

        result
    }

    fn utf8_and_xor(input: &str, prng: &mut PRNGEncoder) -> Vec<i32> {
        let mut utf8: Vec<i32> = Vec::new();
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            let code = c as u32;

            if code < 128 {
                utf8.push(code as i32);
            } else if code < 2048 {
                utf8.push(((code >> 6) | 192) as i32);
                utf8.push(((code & 63) | 128) as i32);
            } else if (code & 0xFC00) == 0xD800 {
                if let Some(&next) = chars.peek() {
                    let next_code = next as u32;
                    if (next_code & 0xFC00) == 0xDC00 {
                        let high = code & 0x3FF;
                        let low = next_code & 0x3FF;
                        let combined = 0x10000 + (high << 10) + low;
                        utf8.push(((combined >> 18) | 240) as i32);
                        utf8.push((((combined >> 12) & 63) | 128) as i32);
                        utf8.push((((combined >> 6) & 63) | 128) as i32);
                        utf8.push(((combined & 63) | 128) as i32);
                        chars.next(); 
                    }
                }
            } else {
                utf8.push(((code >> 12) | 224) as i32);
                utf8.push((((code >> 6) & 63) | 128) as i32);
                utf8.push(((code & 63) | 128) as i32);
            }
        }

        for i in 0..utf8.len() {
            utf8[i] ^= prng.next(false);
        }

        utf8
    }

    fn encoder_tobase64(input_key: &str, buffer: &[i32], prng_main: &mut PRNGEncoder, base_counter: &mut i64) -> String {
        let mut prng_key: PRNGEncoder = PRNGEncoder::new(1809053797 ^ PRNGEncoder::string_hash(input_key), *base_counter, true);

        let mut result: Vec<i32> = Vec::new();
        for &val in buffer {
            result.push(val ^ prng_key.next(false));
        }

        result.push(125 ^ prng_main.next(true) ^ prng_key.next(false));

        PRNGEncoder::custom_base64(&result, base_counter)
    }

    pub fn encode(encoder_parsed: &EncoderParsed, strings_to_encode: Vec<String>, input_key: String) -> String {
        let const_c: i64 = 9959949970;
        let str_key: &str = encoder_parsed.str_key.as_str();
        let dynamic_num: i64 = encoder_parsed.dynamic_num;

        let combined_seed: i64 = const_c ^ PRNGEncoder::string_hash(str_key) ^ dynamic_num;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;

        let mut base_counter: i64 = PRNGEncoder::xorshift32(
            PRNGEncoder::xorshift32(((now >> 3) ^ 11027890091) as i64)
                .wrapping_mul(const_c as i64),
        );

        let mut prng_main: PRNGEncoder = PRNGEncoder::new(combined_seed, base_counter, true);
        let mut buffer: Vec<i32> = Vec::new();

        for value in &strings_to_encode {
            buffer.extend(PRNGEncoder::utf8_and_xor(value.as_str(), &mut prng_main));
        }

        println!("{:?}", buffer);

        let result: String = PRNGEncoder::encoder_tobase64(
            input_key.as_str(),
            &buffer,
            &mut prng_main,
            &mut base_counter,
        );
        result
    }
}