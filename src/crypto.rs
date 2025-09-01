use crate::error::{ErrorKind, Result};
use aes::Aes128;
use aes::cipher::{BlockEncryptMut, KeyInit, block_padding::Pkcs7, generic_array::GenericArray};

type Aes128EcbEnc = ecb::Encryptor<Aes128>;

/// Encrypts password using AES-128-ECB with PKCS7 padding
pub fn encrypt_password(password: &str, aes_key: &[u8]) -> Result<String> {
    let srcs = password.as_bytes();
    let key = GenericArray::from_slice(aes_key);
    let mut buf = [0u8; 128];
    let pt_len = srcs.len();

    buf[..pt_len].copy_from_slice(srcs);

    let ct = Aes128EcbEnc::new(key)
        .encrypt_padded_mut::<Pkcs7>(&mut buf, pt_len)
        .unwrap();
    let base64 = base64_simd::STANDARD;
    Ok(base64.encode_to_string(ct))
}

/// Decodes base64 captcha image
pub fn decode_captcha_image(captcha_b64: &str) -> Result<Vec<u8>> {
    let base64 = base64_simd::STANDARD;
    Ok(base64.decode_to_vec(
        captcha_b64
            .split(',')
            .nth(1)
            .ok_or(ErrorKind::ParseError("Invalid captcha image".to_string()))?,
    )?)
}
