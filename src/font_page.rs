use crate::Args;

use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FontPage {
    time: u64,
    pub contents: Option<String>,
}

impl FontPage {
    pub fn get_font_page(
        url: &str,
        args: Arc<Args>,
        client: &reqwest::blocking::Client,
        cached_pages: Arc<Mutex<HashMap<u64, FontPage>>>,
    ) -> Result<Self, String> {
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        let url_hash = hasher.finish();

        let font_page = cached_pages.lock().unwrap().get(&url_hash).cloned();

        if let Some(font_page) = font_page {
            match font_page.contents.clone() {
                Some(_) => {
                    if args.options.verbose | args.config.verbose_urls {
                        println!("Reading cache (RAM):  {url} ({url_hash})");
                    }
                    return Ok(font_page);
                }
                None => loop {
                    if cached_pages
                        .lock()
                        .unwrap()
                        .get(&url_hash)
                        .is_some_and(|entry| entry.contents.is_none())
                    {
                        thread::sleep(Duration::from_millis(20));
                        continue;
                    }

                    return FontPage::get_font_page(url, args, client, cached_pages);
                },
            }
        }

        cached_pages.lock().unwrap().insert(
            url_hash,
            FontPage {
                time: 0,
                contents: None,
            },
        );

        let cache_file = format!("{}{}", page_cache_dir!(), &url_hash);
        let mut font_page: FontPage =
            toml::from_str(&fs::read_to_string(&cache_file).unwrap_or_default())
                .unwrap_or_default();

        let system_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .div_f64(60.0)
            .as_secs();

        if font_page.contents.is_none()
            || args.options.refresh
            || system_time.wrapping_sub(font_page.time) >= args.config.cache_timeout
        {
            if args.options.verbose | args.config.verbose_urls {
                println!("Updating cache:       {url} ({url_hash})");
            }
            let page = client
                .get(url)
                .header(USER_AGENT, "fin")
                .send()
                .map_err(|e| {
                    cached_pages.lock().unwrap().remove_entry(&url_hash);
                    e.to_string()
                })?;

            font_page.time = system_time;
            font_page.contents = Some(page.text().map_err(|e| {
                cached_pages.lock().unwrap().remove_entry(&url_hash);
                e.to_string()
            })?);

            fs::write(
                &cache_file,
                &toml::to_string(&font_page).map_err(|e| {
                    cached_pages.lock().unwrap().remove_entry(&url_hash);
                    e.to_string()
                })?,
            )
            .map_err(|e| {
                cached_pages.lock().unwrap().remove_entry(&url_hash);
                e.to_string()
            })?;
        } else if args.options.verbose | args.config.verbose_urls {
            println!("Reading cache (disk): {url} ({url_hash})");
        }

        *cached_pages.lock().unwrap().get_mut(&url_hash).unwrap() = font_page.clone();

        Ok(font_page)
    }
}
