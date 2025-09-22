use crate::Installer;

use crate::font_page::FontPage;
use crate::wildcards::*;
use crate::Args;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub enum Source {
    GitHub {
        tag: Option<String>,
        author: String,
        project: String,
    },
    Webpage {
        tag: Option<String>,
        url: String,
    },
    Direct {
        tag: Option<String>,
        url: String,
    },
    None,
}

impl Source {
    pub fn validate(&mut self, file: &str, name: &str) -> Result<(), String> {
        match self {
            Source::GitHub {
                author, project, ..
            } => {
                if author.is_empty() {
                    return Err(format!("{name}: Unspecified GitHub author"));
                }
                if project.is_empty() {
                    return Err(format!("{name}: Unspecified GitHub project"));
                }
                if author.contains(['/', '?', '#', '&', '$', '\\']) {
                    return Err(format!("{name}: GitHub author \"{author}\" is invalid"));
                }
                if project.contains(['/', '?', '#', '&', '$', '\\']) {
                    return Err(format!("{name}: GitHub project \"{project}\" is invalid"));
                }
            }
            Source::Webpage { tag, url } => {
                if !match_wildcard(url, "*://*.*/*") {
                    return Err(format!("{name}: Invalid URL: \"{url}\""));
                }
                if let Some(tag) = tag.as_mut() {
                    *url = url.replace("$tag", tag);
                } else if url.contains("$tag") {
                    return Err(format!("{name}: Use of missing field: `$tag`"));
                }
            }
            Source::Direct { url, .. } => {
                if !url.ends_with("$file") {
                    return Err(format!("{name}: Direct URLs must end with `$file`"));
                }

                // TODO: Get the redirected URL for direct links
                *url = url.replace("$file", file);
            }
            Source::None => return Err(format!("{name}: A valid source must be provided")),
        }
        Ok(())
    }

    pub fn validate_tag(&mut self, override_version: Option<&str>) {
        match self {
            Self::GitHub { tag, .. } => {
                if override_version.is_some() {
                    *tag = override_version.map(|v| v.to_string());
                } else if tag.is_none() {
                    *tag = Some("latest".to_string());
                };
            }
            Self::Webpage { tag, .. } | Self::Direct { tag, .. } => {
                override_version.inspect(|v| *tag = Some(v.to_string()));
            }
            Self::None => (),
        }
    }

    pub fn into_direct_url(
        &mut self,
        installer: &mut Installer,
        args: &Arc<Args>,
        override_version: Option<&str>,
        cached_pages: Arc<Mutex<HashMap<String, FontPage>>>,
    ) -> Result<(), String> {
        match self {
            Source::GitHub {
                author, project, ..
            } => {
                *self = Source::Webpage {
                    url: format!("https://api.github.com/repos/{author}/{project}/releases/$tag"),
                    tag: if let Source::GitHub { tag, .. } = self.take() {
                        tag
                    } else {
                        panic!() // Unreachable
                    },
                };
                self.validate(installer.action.ref_file()?, &installer.installer_name)?;
                self.into_direct_url(installer, args, override_version, cached_pages)
            }
            Source::Webpage { url, .. } => {
                installer.font_page = Self::get_font_page(args, url, cached_pages)?.contents;
                let url = Self::find_direct_link(
                    installer.font_page.as_ref().unwrap(),
                    installer.action.ref_file()?,
                    &installer.installer_name,
                )?;
                let Source::Webpage { tag, .. } = self.take() else {
                    panic!() // Unreachable
                };
                *self = Self::Direct { url, tag };
                Ok(())
            }
            Source::Direct { .. } | Source::None => Ok(()),
        }
    }
    fn get_font_page(
        args: &Arc<Args>,
        webpage_url: &str,
        cached_pages: Arc<Mutex<HashMap<String, FontPage>>>,
    ) -> Result<FontPage, String> {
        let reqwest_client = reqwest::blocking::Client::new();
        FontPage::get_font_page(webpage_url, args, &reqwest_client, cached_pages)
    }

    /// Returns a direct link to the `file` found within `font_page`
    pub fn find_direct_link(
        font_page_contents: &str,
        file: &str,
        name: &str,
    ) -> Result<String, String> {
        font_page_contents
            .split('"')
            .find_map(|line| wildcard_substring(line, &(String::from("https://*") + file), b""))
            .map_or_else(
                || {
                    Err(format!(
                        "{name}: File \"{file}\" could not be found within the webpage"
                    ))
                },
                |link| Ok(link.to_string()),
            )
    }

    pub fn ref_direct_url(&self) -> Result<&str, String> {
        match self {
            Source::Direct { url, .. } => Ok(url),
            source => Err(format!("Cannot use as `Direct` URL: `{source:?}`")),
        }
    }

    pub fn ref_webpage_url(&self) -> Result<&str, String> {
        match self {
            Source::Webpage { url, .. } => Ok(url),
            source => Err(format!("Cannot use as `Webpage` URL: `{source:?}`")),
        }
    }

    pub fn ref_tag(&self) -> Result<Option<&str>, String> {
        match self {
            Source::GitHub { tag, .. }
            | Source::Webpage { tag, .. }
            | Source::Direct { tag, .. } => Ok(tag.as_deref()),
            Source::None => Err(format!("Cannot obtain field `tag` from `{:?}`", self)),
        }
    }

    pub const fn take(&mut self) -> Self {
        let mut output = Self::None;
        std::mem::swap(self, &mut output);
        output
    }
}
