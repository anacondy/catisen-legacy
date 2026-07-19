// Catisen Ad & Tracker Blocker
// Parses raw .txt filter lists (like uBlock Origin's EasyList) and drops malicious network requests.

use adblock::engine::Engine;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

pub static GLOBAL_ADBLOCKER: OnceLock<Mutex<AdBlocker>> = OnceLock::new();

pub fn get_adblocker() -> &'static Mutex<AdBlocker> {
    GLOBAL_ADBLOCKER.get_or_init(|| {
        let mut adb = AdBlocker::new();
        // Load the default file, if not found it falls back to built-ins
        let _ = adb.load_filter_list("easylist.txt");
        Mutex::new(adb)
    })
}

pub struct AdBlocker {
    engine: Option<Engine>,
    pub is_enabled: bool,
}

impl AdBlocker {
    pub fn new() -> Self {
        AdBlocker {
            engine: None,
            is_enabled: true, // Ad-blocking is ON by default
        }
    }

    pub fn load_filter_list(&mut self, file_path: &str) -> std::io::Result<()> {
        let path = Path::new(file_path);
        
        let filter_rules = if path.exists() {
            fs::read_to_string(path)?
        } else {
            println!("WARNING: Filter list {} not found. Falling back to default list.", file_path);
            self.get_built_in_defaults().join("\n")
        };

        // Convert the string into owned lines and pass to the Engine builder
        let parsed_rules: Vec<String> = filter_rules.lines().map(|line| line.to_owned()).collect();
        
        let engine = Engine::from_rules(&parsed_rules, adblock::lists::ParseOptions {
            ..Default::default()
        });

        self.engine = Some(engine);
        println!("AD BLOCKER: Fully armed. Rules loaded from: {}", if path.exists() { file_path } else { "built-in lists" });
        Ok(())
    }

    fn get_built_in_defaults(&self) -> Vec<String> {
        vec![
            "||google-analytics.com^".to_string(),
            "||doubleclick.net^".to_string(),
            "||facebook.net^".to_string(),
            "||pixel.facebook.com^".to_string(),
            "||ads.twitter.com^".to_string(),
            "||scorecardresearch.com^".to_string(),
            "||taboola.com^".to_string(),
            "||outbrain.com^".to_string(),
            "||criteo.com^".to_string()
        ]
    }

    pub fn should_block_request(&self, url: &str) -> bool {
        if !self.is_enabled {
            return false;
        }

        if let Some(engine) = &self.engine {
            // Provide context ("document" normally) to the adblock crate for rule matching
            if let Ok(request) = adblock::request::Request::new(url, url, "document") {
                let result = engine.check_network_request(&request);
                if result.matched {
                    println!("BLOCKED: Dropped malicious connection to {}", url);
                    return true;
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adblock_defaults() {
        let mut blocker = AdBlocker::new();
        // Load with a dummy file that doesn't exist to trigger built-in defaults
        let _ = blocker.load_filter_list("nonexistent_test_file.txt");
        
        assert!(blocker.should_block_request("https://google-analytics.com/some/tracker.js"), "Should block GA");
        assert!(blocker.should_block_request("http://ads.twitter.com/pixel"), "Should block Twitter ads");
        assert!(!blocker.should_block_request("https://en.wikipedia.org"), "Should not block clean site");
    }
}
