// Catisen Download Manager - Multi-threaded, Pause/Resume
// Uses libcurl for non-throttled bare-metal download speeds.

use curl::easy::Easy;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct DownloadManager {
    pub downloads: Arc<Mutex<Vec<Download>>>,
}

#[derive(Debug, Clone)]
pub struct Download {
    pub id: usize,
    pub url: String,
    pub path: String,
    pub progress_percent: Arc<Mutex<f32>>,
    pub total_bytes: Arc<Mutex<f64>>,
    pub downloaded_bytes: Arc<Mutex<f64>>,
    pub is_paused: Arc<Mutex<bool>>,
    pub is_completed: Arc<Mutex<bool>>,
}

impl DownloadManager {
    pub fn new() -> Self {
        DownloadManager {
            downloads: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Spawns a dedicated OS thread for a high-speed download with progress tracking
    pub fn start_download(&mut self, url: &str, path: &str) {
        let mut downloads_lock = self.downloads.lock().unwrap();
        let new_id = downloads_lock.len();
        
        let path_clone = path.to_string();
        
        let progress_percent = Arc::new(Mutex::new(0.0));
        let total_bytes = Arc::new(Mutex::new(0.0));
        let downloaded_bytes = Arc::new(Mutex::new(0.0));
        let is_paused = Arc::new(Mutex::new(false));
        let is_completed = Arc::new(Mutex::new(false));

        let download = Download {
            id: new_id,
            url: url.to_string(),
            path: path.to_string(),
            progress_percent: progress_percent.clone(),
            total_bytes: total_bytes.clone(),
            downloaded_bytes: downloaded_bytes.clone(),
            is_paused: is_paused.clone(),
            is_completed: is_completed.clone(),
        };
        downloads_lock.push(download.clone());

        let url_clone = url.to_string();

        thread::spawn(move || {
            let mut easy = Easy::new();
            easy.url(&url_clone).unwrap();
            easy.follow_location(true).unwrap();

            // Setup real-time progress tracking
            easy.progress(true).unwrap();
            let p_pause = is_paused.clone();
            let p_percent = progress_percent.clone();
            let p_total = total_bytes.clone();
            let p_down = downloaded_bytes.clone();
            
            easy.progress_function(move |total_dl, downloaded, _, _| {
                if *p_pause.lock().unwrap() {
                    return false; // Returning false brutally aborts libcurl
                }
                
                if total_dl > 0.0 {
                    let mut percent = p_percent.lock().unwrap();
                    let mut tot = p_total.lock().unwrap();
                    let mut dwn = p_down.lock().unwrap();
                    
                    *percent = (downloaded / total_dl) as f32;
                    *tot = total_dl;
                    *dwn = downloaded;
                }
                true
            }).unwrap();

            // Check if file exists to resume, otherwise create new
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path_clone)
                .unwrap();

            let file_metadata = file.metadata().unwrap();
            let starting_size = file_metadata.len();
            
            if starting_size > 0 {
                println!("🚀 RESUMING download for {} at byte {}", path_clone, starting_size);
                easy.resume_from(starting_size).unwrap();
            } else {
                println!("🚀 STARTING high-speed download to {}", path_clone);
            }

            let write_path_clone = path_clone.clone();
            easy.write_function(move |data| {
                // If the user hit pause in the UI, abort this chunk so the thread yields
                if *is_paused.lock().unwrap() {
                    println!("⏸️ PAUSED download at {}", write_path_clone);
                    return Ok(0); // Returning 0 bytes processed tells libcurl to abort
                }
                
                // Write data to hard-drive at raw disk speed
                file.write_all(data).unwrap();
                Ok(data.len())
            }).unwrap();

            match easy.perform() {
                Ok(_) => {
                    println!("✅ COMPLETED downloaded to {}", path_clone);
                    *is_completed.lock().unwrap() = true;
                    *progress_percent.lock().unwrap() = 1.0;
                },
                Err(e) => {
                    if !e.is_write_error() && !e.is_aborted_by_callback() {
                        println!("❌ ERROR downloading: {}", e);
                    } else if e.is_aborted_by_callback() {
                        println!("⏸️ PAUSE ACKNOWLEDGED for {}", path_clone);
                    }
                }
            }
        });
    }

    /// Sends a signal to immediately halt the download thread
    pub fn pause_download(&mut self, id: usize) {
        if let Some(download) = self.downloads.lock().unwrap().get_mut(id) {
            let mut pause_signal = download.is_paused.lock().unwrap();
            *pause_signal = true;
        }
    }

    /// Spawns a new thread that resumes appending to the exact byte left off
    pub fn resume_download(&mut self, id: usize) {
        let mut url_clone = String::new();
        let mut path_clone = String::new();

        if let Some(download) = self.downloads.lock().unwrap().get_mut(id) {
            let mut pause_signal = download.is_paused.lock().unwrap();
            *pause_signal = false;
            
            url_clone = download.url.clone();
            path_clone = download.path.clone();
        }
        
        // Re-trigger the start logic which will automatically detect the file size and resume
        if !url_clone.is_empty() {
            self.start_download(&url_clone, &path_clone);
        }
    }
    
    /// Get current progress of a specific download ID between 0.0 and 1.0
    pub fn get_progress(&self, id: usize) -> Option<f32> {
        let downloads = self.downloads.lock().unwrap();
        if let Some(dl) = downloads.get(id) {
            let prog = dl.progress_percent.lock().unwrap();
            return Some(*prog);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_manager_lifecycle() {
        let mut dm = DownloadManager::new();
        assert_eq!(dm.downloads.lock().unwrap().len(), 0);
        
        let url = "https://example.com/test.bin";
        let path = "test.bin";
        dm.start_download(url, path);
        // Wait briefly for the thread to register
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(dm.downloads.lock().unwrap().len(), 1);
        
        let id_to_test = 0;
        dm.pause_download(id_to_test);
        {
            let downloads = dm.downloads.lock().unwrap();
            let dl = &downloads[id_to_test];
            let is_paused = dl.is_paused.lock().unwrap();
            assert!(*is_paused);
        }
        
        dm.resume_download(id_to_test);
    }
}