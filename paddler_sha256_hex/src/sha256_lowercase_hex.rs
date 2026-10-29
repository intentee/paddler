use sha2::Digest;
use sha2::Sha256;

const LOWERCASE_HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

#[must_use]
pub fn sha256_lowercase_hex(input: &[u8]) -> String {
    Sha256::digest(input)
        .iter()
        .flat_map(|digest_byte| {
            [
                LOWERCASE_HEX_DIGITS[usize::from(digest_byte >> 4)],
                LOWERCASE_HEX_DIGITS[usize::from(digest_byte & 0x0f)],
            ]
        })
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::sha256_lowercase_hex;

    #[test]
    fn encodes_the_digest_high_nibble_first_in_lowercase_hex() {
        assert_eq!(
            sha256_lowercase_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
