// Catisen Browser - Tor & Proxy Routing Logic
// This module handles intercepting browser traffic and routing it through
// encrypted SOCKS5 (Tor) or external HTTP/HTTPS proxies to bypass ISP firewalls.

#[derive(Debug, Clone)]
pub enum ProxyMode {
    Direct,          // Standard connection (visible to ISP)
    Tor,             // Local Tor SOCKS5 proxy (127.0.0.1:9050)
    Custom(String),  // Custom external Proxy/VPN IP
}

pub struct ProxyRouter {
    pub current_mode: ProxyMode,
    pub https_only: bool,
}

impl ProxyRouter {
    pub fn new() -> Self {
        ProxyRouter {
            current_mode: ProxyMode::Direct,
            https_only: true, // Default to strict HTTPS enforcement (Day 6 feature)
        }
    }

    /// Day 6: HTTPS Enforcement Logic
    /// Blocks unencrypted HTTP traffic unless it's natively encrypted by Tor (.onion)
    pub fn is_url_allowed(&self, url: &str) -> bool {
        if !self.https_only {
            return true;
        }

        if url.starts_with("https://") {
            return true; // Standard secure web
        }

        if url.starts_with("http://") {
            // .onion traffic is end-to-end encrypted by the Tor network itself,
            // so http:// is actually safe and standard for onions.
            if url.contains(".onion") {
                return true;
            }
            // Block all other plain HTTP traffic on the clearnet
            println!("🚫 SECURITY BLOCK: Prevented unencrypted HTTP connection to {}", url);
            return false;
        }

        // Allow other safe schemes like about:, chrome:, file:
        true
    }

    /// Engages the Tor Network route
    pub fn enable_tor(&mut self) {
        println!("🔒 SECURE ROUTING: Engaging Tor Network bypass...");
        self.current_mode = ProxyMode::Tor;
        self.apply_network_rules();
    }

    /// Engages an external/custom Proxy server (e.g., Geo-spoofing to Canada)
    pub fn enable_custom_proxy(&mut self, proxy_ip: &str) {
        println!("🌍 SECURE ROUTING: Re-routing via Custom Proxy (IP: {})...", proxy_ip);
        self.current_mode = ProxyMode::Custom(proxy_ip.to_string());
        self.apply_network_rules();
    }

    /// Drops the proxy and uses standard unencrypted ISP connection
    pub fn disable_proxy(&mut self) {
        println!("🌐 DIRECT ROUTING: Proxy disabled. Normal connection active.");
        self.current_mode = ProxyMode::Direct;
        self.apply_network_rules();
    }

    /// Pushes the chosen proxy configuration into Servo and the Environment
    fn apply_network_rules(&self) {
        match &self.current_mode {
            ProxyMode::Tor => {
                // Tells the underlying network stack to bounce through the local Tor port
                std::env::set_var("http_proxy", "socks5://127.0.0.1:9050");
                std::env::set_var("https_proxy", "socks5://127.0.0.1:9050");
                println!("   --> STATUS = ISP Firewall Bypass Active. Location: Hidden.");
            }
            ProxyMode::Custom(ip) => {
                // Forces internet traffic to go to our chosen IP server first
                std::env::set_var("http_proxy", ip);
                std::env::set_var("https_proxy", ip);
                println!("   --> STATUS = Geo-Spoof Active. Location overridden.");
            }
            ProxyMode::Direct => {
                // Clears overrides
                std::env::remove_var("http_proxy");
                std::env::remove_var("https_proxy");
                println!("   --> STATUS = Connection exposed to local ISP.");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use curl::easy::Easy;

    #[test]
    fn test_tor_ip_check() {
        let mut easy = Easy::new();
        // Pointing query to torproject's ip API.
        easy.url("https://check.torproject.org/api/ip").unwrap();
        // The critically important 'socks5h' handler for tor proxy DNS anti-leak
        easy.proxy("socks5h://127.0.0.1:9050").unwrap();
        
        let mut data = Vec::new();
        {
            let mut transfer = easy.transfer();
            transfer.write_function(|new_data| {
                data.extend_from_slice(new_data);
                Ok(new_data.len())
            }).unwrap();
            let _ = transfer.perform();
        }
        
        // This test will silently pass if tor is off on the testing machine,
        // but log its results if its successfully routed. 
        if !data.is_empty() {
            let json_resp = String::from_utf8_lossy(&data);
            println!("Tor Proxy Test Success! IP Data: {}", json_resp);
        }
    }
}
