use aes::Aes256;
use block_modes::{BlockMode, Ecb, block_padding::ZeroPadding};

use std::fmt::Display;
use std::io::SeekFrom;
use std::ops::Add;

use crate::archive::FArchive;
use crate::{Result, ParserError};

const KEY_SIZE: usize = 32;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct FAesKey {
    pub(crate) key: [u8; KEY_SIZE]
}

impl Display for FAesKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl FAesKey {

    pub fn from_base64(base64: &str) -> Result<Self> {
        let decoded = base64::decode(base64)?;
        Ok(Self {
            key: decoded.try_into().unwrap()
        })
    } 

    pub fn to_hex(&self) -> String {
        "0x".to_owned().add(&hex::encode(&self.key))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.key
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = vec![0u8; self.key.len()];
        result.copy_from_slice(&self.key);

        result
    }

    pub(crate) fn decrypt<Ar>(&self, archive: &mut Ar, offset: u64, len: usize) -> Result<()> 
    where Ar: FArchive {
        let cipher = Ecb::<Aes256, ZeroPadding>::new_from_slices(&self.key, Default::default())?;

        archive.seek(SeekFrom::Start(offset))?;
        let mut encrypted = vec![0; len];
        archive.read_bytes_vec(&mut encrypted)?;

        let start = usize::try_from(offset)?;
        let end = start + len;
        let block = &mut archive.get_mut()[start..end];
        let decrypted = cipher.decrypt_vec(encrypted.as_mut_slice())?;
        if decrypted.len() != block.len() {
            return Err(Box::new(ParserError::new("decrypted block size != encrypted block size")));
        }

        block.copy_from_slice(decrypted.as_slice());
        Ok(())
    }

}