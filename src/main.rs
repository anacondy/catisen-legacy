use crate::network::error::FetchError;

#[cfg(feature = "gtk_ui")]
mod settings_ui;
mod network;
mod privacy;
mod tor_proxy;
mod tab_isolation;
mod text_mode;
mod ublock_integration;
mod media_extractor;
mod libcurl_download_manager;
mod history;
mod permissions;
mod sandbox;
mod extensions;
mod sync_chain;
mod servo_renderer;
mod config;

#[cfg(feature = "gtk_ui")]
mod gtk_ui;

#[cfg(feature = "egui_ui")]
mod ui;
mod debug_panel;

fn main() {
    // Install default crypto provider for rustls to prevent panics inside servo networking
    let _ = rustls::crypto::ring::default_provider().install_default();

        let config = crate::config::CatisenConfig::load_or_create().unwrap_or_else(|e| {
        eprintln!("Failed to load config: {}, using default.", e);
        crate::config::CatisenConfig::default()
    });
    let mut tor_enabled = config.use_tor_by_default;
    
    if std::env::var("CATISEN_TOR_PROXY").is_err() {
        std::env::set_var("CATISEN_TOR_PROXY", &config.tor_proxy_url);
    }

    let args: Vec<String> = std::env::args().collect();
    let mut initial_url = None;
    
    let mut i = 1;
    let mut force_test_mode = false;
    while i < args.len() {
        match args[i].as_str() {
            "--url" => {
                if i + 1 < args.len() {
                    initial_url = Some(args[i+1].clone());
                    i += 1;
                }
            }
            "--tor" => tor_enabled = true,
            "--headless" | "--test" => force_test_mode = true,
            _ => {}
        }
        i += 1;
    }

    #[cfg(feature = "gtk_ui")]
    {
        println!("Starting Catisen with GTK4 UI...");
        gtk_ui::run();
    }

    #[cfg(feature = "egui_ui")]
    {
        let test_mode = force_test_mode || std::env::var("CATISEN_TEST_MODE")
            .map(|v| v.trim() == "1")
            .unwrap_or(false);

        let mut viewport = eframe::egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_title("Catisen");
            
        if test_mode {
            println!("Catisen is in test mode. Running headless workflow directly.");
            ui::app::run_headless(initial_url.clone(), tor_enabled);
            return;
        }

        let native_options = eframe::NativeOptions {
            viewport,
            vsync: false,
            ..Default::default()
        };

        println!("Starting Catisen with EGUI Native UI...");
        let _ = ui::app::run(initial_url, tor_enabled, native_options);
        
        // Try hiding windows again after startup
        if test_mode {
            std::thread::sleep(std::time::Duration::from_millis(50));
            ui::headless_window::ensure_headless_mode();
        }
    }

    #[cfg(not(any(feature = "gtk_ui", feature = "egui_ui")))]
    {
        println!("No UI framework selected! Please compile with --features egui_ui or --features gtk_ui");
    }
}
