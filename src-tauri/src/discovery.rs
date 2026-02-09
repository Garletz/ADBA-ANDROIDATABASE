//! mDNS service discovery for LAN visibility
//! 
//! Registers ADBA as a service on the local network so client apps can discover it

use crate::error::AdbaError;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;
use tracing::info;

/// Service type for ADBA discovery
const SERVICE_TYPE: &str = "_adba._tcp.local.";
const SERVICE_NAME: &str = "ADBA Database Server";

/// Register ADBA as an mDNS service on the local network
pub fn register_service(port: u16, pairing_code: &str) -> Result<(), AdbaError> {
    // Create mDNS daemon
    let mdns = ServiceDaemon::new()
        .map_err(|e| AdbaError::Discovery(format!("Failed to create mDNS daemon: {}", e)))?;
    
    // Get hostname
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "adba-host".to_string());
    
    let instance_name = format!("{}-{}", SERVICE_NAME, &pairing_code[..4]);
    
    // Create service properties
    let mut properties = HashMap::new();
    properties.insert("version".to_string(), "0.1.0".to_string());
    properties.insert("protocol".to_string(), "postgresql".to_string());
    properties.insert("pairing_prefix".to_string(), pairing_code[..2].to_string());
    
    // Create service info
    let service = ServiceInfo::new(
        SERVICE_TYPE,
        &instance_name,
        &format!("{}.local.", hostname),
        "",  // Will use default IP
        port,
        properties,
    ).map_err(|e| AdbaError::Discovery(format!("Failed to create service info: {}", e)))?;
    
    // Register the service
    mdns.register(service)
        .map_err(|e| AdbaError::Discovery(format!("Failed to register mDNS service: {}", e)))?;
    
    info!(
        "Registered mDNS service '{}' on port {} (pairing prefix: {})",
        instance_name, port, &pairing_code[..2]
    );
    
    // Keep the daemon alive by spawning a background task
    std::thread::spawn(move || {
        // The daemon needs to stay alive to maintain registration
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });
    
    Ok(())
}

/// Scan for other ADBA instances on the network
pub async fn discover_services() -> Result<Vec<DiscoveredService>, AdbaError> {
    let mdns = ServiceDaemon::new()
        .map_err(|e| AdbaError::Discovery(format!("Failed to create mDNS daemon: {}", e)))?;
    
    let receiver = mdns.browse(SERVICE_TYPE)
        .map_err(|e| AdbaError::Discovery(format!("Failed to browse: {}", e)))?;
    
    let mut services = Vec::new();
    
    // Collect services for a short time
    let timeout = std::time::Duration::from_secs(3);
    let start = std::time::Instant::now();
    
    while start.elapsed() < timeout {
        if let Ok(event) = receiver.try_recv() {
            match event {
                mdns_sd::ServiceEvent::ServiceResolved(info) => {
                    services.push(DiscoveredService {
                        name: info.get_fullname().to_string(),
                        host: info.get_hostname().to_string(),
                        port: info.get_port(),
                        addresses: info.get_addresses().iter().map(|a| a.to_string()).collect(),
                    });
                }
                _ => {}
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    
    Ok(services)
}

/// A discovered ADBA service on the network
#[derive(Debug, Clone)]
pub struct DiscoveredService {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub addresses: Vec<String>,
}
