use crate::format_size;
use crate::source::Source;
use crate::{bar::ProgressBar, file_action::FileAction};

use serde::Deserialize;
use sha2::digest::generic_array::ArrayLength;
use sha2::digest::{FixedOutput, HashMarker, OutputSizeUser, Update};
use sha2::{Digest, Sha224, Sha256, Sha384, Sha512};
use std::io::{Read, Write, stdout};
use std::ops::Add;

#[derive(Debug, Deserialize)]
pub enum Checksum {
    SHA224 { file: Option<String> },
    SHA256 { file: Option<String> },
    SHA384 { file: Option<String> },
    SHA512 { file: Option<String> },
}
impl Checksum {
    /// Obtains the checksum and assigns it to `file`
    pub async fn obtain(
        &mut self,
        font_page: Option<String>,
        tag: Option<&str>,
        reqwest_client: &reqwest::Client,
        installer_name: &str,
    ) -> Result<(), String> {
        match self {
            Checksum::SHA224 { file }
            | Checksum::SHA256 { file }
            | Checksum::SHA384 { file }
            | Checksum::SHA512 { file } => {
                let Some(file) = file.as_mut() else {
                    *file = font_page;
                    return Ok(());
                };
                FileAction::validate_file(file, tag, installer_name)?;

                let file_link = Source::find_direct_link(
                    &font_page.expect("Font page is not available"),
                    file,
                    installer_name,
                )?;

                *file = reqwest_client
                    .get(&file_link)
                    .send()
                    .await
                    .map_err(|e| e.to_string())?
                    .text()
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(())
            }
        }
    }

    /// Computes the hash sum of `data` and compares it to `self.file`.
    /// Returns `Ok` if the checksums match, or `Err` if they do not.
    pub fn check(&mut self, data: &[u8], data_size: f64, source: &Source) -> Result<(), String> {
        let filename = source
            .ref_direct_url()?
            .rsplit_once('/')
            .unwrap_or_default()
            .1;
        match self {
            Self::SHA224 { file } => {
                print!("… Verifying:   {filename}");
                let _ = stdout().flush();
                Self::sha_check(file, Sha224::new(), data, data_size, filename)
            }
            Self::SHA256 { file } => {
                print!("… Verifying:   {filename}");
                let _ = stdout().flush();
                Self::sha_check(file, Sha256::new(), data, data_size, filename)
            }
            Self::SHA384 { file } => {
                print!("… Verifying:   {filename}");
                let _ = stdout().flush();
                Self::sha_check(file, Sha384::new(), data, data_size, filename)
            }
            Self::SHA512 { file } => {
                print!("… Verifying:   {filename}");
                let _ = stdout().flush();
                Self::sha_check(file, Sha512::new(), data, data_size, filename)
            }
        }
    }

    /// Computes the hash sum for the input `data`, and compares it to the `expected_sum`.
    /// Returns `Ok` if the `expected_sum` contains the newly calculated sum, or `Err`
    /// if it does not.
    fn sha_check<H>(
        expected_sum: &mut Option<String>,
        mut hasher: H,
        mut data: &[u8],
        data_size: f64,
        filename: &str,
    ) -> Result<(), String>
    where
        H: FixedOutput + Default + Update + HashMarker,
        <H as OutputSizeUser>::OutputSize: Add,
        <<H as OutputSizeUser>::OutputSize as Add>::Output: ArrayLength<u8>,
    {
        let mut progress_bar = ProgressBar::new("Verifying:");
        let bytes_total_text = format_size(data_size);
        let mut bytes_progress = 0;

        let mut buffer = [0; 8192];
        loop {
            let bytes_read = data.read(&mut buffer).map_err(|e| {
                progress_bar.fail();
                e.to_string()
            })?;

            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);

            bytes_progress += bytes_read;

            progress_bar.update_progress(
                bytes_progress as f64 / data_size,
                &format!(
                    " {} / {bytes_total_text}",
                    format_size(bytes_progress as f64)
                ),
            );
        }

        let sum = hasher.finalize();
        if expected_sum.as_ref().unwrap().contains(&format!("{sum:x}")) {
            progress_bar.pass();
            Ok(())
        } else {
            progress_bar.fail();
            Err(format!("{filename}: Integrity check failed: sum mismatch"))
        }
    }
}
