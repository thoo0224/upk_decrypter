use aes::Aes256;
use block_modes::{BlockMode, Ecb, block_padding::ZeroPadding};

use std::io::SeekFrom;
use std::ops::Add;

use crate::archive::FArchive;
use crate::{Result, ParserError};

const KEY_SIZE: usize = 32;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)] // display to hex
pub struct FAesKey {
    pub(crate) key: [u8; KEY_SIZE]
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

    pub fn decrypt<Ar>(&self, archive: &mut Ar, offset: u64, len: usize) -> Result<()> 
    where Ar: FArchive {
        let cipher = Ecb::<Aes256, ZeroPadding>::new_from_slices(&self.key, Default::default())?;

        archive.seek(SeekFrom::Start(offset))?;
        let mut encrypted = vec![0; len];
        archive.read_bytes_vec(&mut encrypted)?;

        let start = offset as usize;
        let end = start+len as usize;
        let block = &mut archive.get_mut()[start..end];
        let decrypted = cipher.decrypt_vec(encrypted.as_mut_slice())?;
        if decrypted.len() != block.len() {
            return Err(Box::new(ParserError::new("decrypted block size != encrypted block size")));
        }

        block.copy_from_slice(decrypted.as_slice());

        //log::info!("decrypted block of {} bytes", decrypted.len());
        Ok(())
    }

}