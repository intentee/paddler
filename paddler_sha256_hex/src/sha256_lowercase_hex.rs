use sha2::Digest;
use sha2::Sha256;
use sha2::digest::consts::U16;

#[must_use]
pub fn sha256_lowercase_hex(input: &[u8]) -> String {
    let (leading_digest_half, trailing_digest_half) = Sha256::digest(input).split::<U16>();

    format!(
        "{:032x}{:032x}",
        u128::from_be_bytes(leading_digest_half.into()),
        u128::from_be_bytes(trailing_digest_half.into())
    )
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
