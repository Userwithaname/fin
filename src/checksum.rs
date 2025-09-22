use crate::format_size;
use crate::source::Source;
use crate::{bar::ProgressBar, file_action::FileAction};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::{stdout, Read, Write};

#[derive(Debug, Deserialize)]
pub enum Checksum {
    SHA256 { file: Option<String> },
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
            Checksum::SHA256 { file } => {
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

    /// Computes the hash sum of `data` and compares it to `file`.
    /// Returns `Ok` if file contains the same sum string, or `Err` if not
    pub fn check(&mut self, data: &[u8], data_size: f64, source: &Source) -> Result<(), String> {
        let mut data = data;
        let filename = source
            .ref_direct_url()?
            .split('/')
            .next_back()
            .unwrap_or_default();
        match self {
            Self::SHA256 { file } => {
                print!("… Verifying:   {filename}");
                let _ = stdout().flush();

                let bytes_total_text = format_size(data_size);
                let mut bytes_progress = 0;
                let mut progress_bar = ProgressBar::new("Verifying:");

                let mut hasher = Sha256::new();
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
                if file.as_ref().unwrap().contains(&format!("{sum:x}")) {
                    progress_bar.pass();
                    Ok(())
                } else {
                    progress_bar.fail();
                    Err(format!("{filename}: Integrity check failed: sum mismatch"))
                }
            }
        }
    }
}
