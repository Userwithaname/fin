use crate::cache_dir;
use crate::home_dir;
use crate::Args;

use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FontPage {
    time: u64,
    pub contents: Option<String>,
}

impl FontPage {
    pub fn get_font_page(
        url: &str,
        args: &Args,
        client: &reqwest::blocking::Client,
        cached_pages: &mut HashMap<u64, FontPage>,
    ) -> Result<Self, String> {
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        let url_hash = hasher.finish();
        if let Some(font_page) = cached_pages.get(&url_hash) {
            if args.options.verbose {
                println!("Reading cache (memory): {url} ({url_hash})");
            }
            return Ok(font_page.clone());
        }

        let cache_file = format!("{}{}", cache_dir!(), &url_hash);
        let mut cache: FontPage =
            toml::from_str(&fs::read_to_string(&cache_file).unwrap_or_default())
                .unwrap_or_default();

        let system_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .div_f64(60.0)
            .as_secs();

        if cache.contents.is_none()
            || args.options.refresh
            || system_time.wrapping_sub(cache.time) >= args.config.cache_timeout
        {
            if args.options.verbose {
                println!("Updating cache:         {url} ({url_hash})");
            }
            let page = client
                .get(url)
                .header(USER_AGENT, "fin")
                .send()
                .map_err(|e| e.to_string())?;

            cache.time = system_time;
            cache.contents = Some(page.text().map_err(|e| e.to_string())?);

            fs::write(
                &cache_file,
                &toml::to_string(&cache).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        } else if args.options.verbose {
            println!("Reading cache (disk):   {url} ({url_hash})");
        }

        cached_pages.insert(url_hash, cache.clone());
        Ok(cache)
    }
}
