// Jackson Coxson

use axum::{body::Bytes, http::StatusCode, response::Html};
use axum_client_ip::SecureClientIp;
use log::info;
use plist::Dictionary;
use sha2::Digest;
use sqlite::State;
use std::net::{IpAddr, Ipv6Addr};

/// Check to make sure the Wireguard interface exists
pub fn check_wireguard() {
    let wireguard_config_name =
        std::env::var("WIREGUARD_CONFIG_NAME").unwrap_or("jitstreamer".to_string());
    let wireguard_conf = format!("/etc/wireguard/{wireguard_config_name}.conf");
    let wireguard_port = std::env::var("WIREGUARD_PORT")
        .unwrap_or("51869".to_string())
        .parse::<u16>()
        .unwrap_or(51869);
    let wireguard_server_address =
        std::env::var("WIREGUARD_SERVER_ADDRESS").unwrap_or("fd00::/128".to_string());

    if !std::fs::exists(&wireguard_conf).unwrap() {
        let key = wg_config::WgKey::generate_private_key().expect("failed to generate key");
        let interface = wg_config::WgInterface::new(
            key,
            wireguard_server_address.parse().unwrap(),
            Some(wireguard_port),
            None,
            None,
            None,
        )
        .unwrap();

        wg_config::WgConf::create(wireguard_conf.as_str(), interface, None)
            .expect("failed to create config");

        info!("Created new Wireguard config");

        // Run wg-quick up jitstreamer
        let _ = std::process::Command::new("bash")
            .arg("-c")
            .arg(format!("wg-quick up {wireguard_config_name}"))
            .output()
            .expect("failed to execute process");
    }
}

/// Takes the plist in bytes, and returns either the pairing file in return or an error message
pub async fn register(
    client_ip: SecureClientIp,
    plist_bytes: Bytes,
) -> Result<Bytes, (StatusCode, &'static str)> {
    let plist = match plist::from_bytes::<Dictionary>(plist_bytes.as_ref()) {
        Ok(plist) => plist,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "bad plist")),
    };
    let udid = match plist.get("UDID") {
        Some(plist::Value::String(udid)) => udid,
        _ => return Err((StatusCode::BAD_REQUEST, "no UDID")),
    }
    .to_owned();

    let cloned_udid = udid.clone();
    // Reverse lookup the device to see if we already have an IP for it
    let ip = match tokio::task::spawn_blocking(move || {
        let db = match sqlite::open("jitstreamer.db") {
            Ok(db) => db,
            Err(e) => {
                info!("Failed to open database: {:?}", e);
                return None;
            }
        };

        // Get the device from the database
        let query = "SELECT ip FROM devices WHERE udid = ?";
        let mut statement = match crate::db::db_prepare(&db, query) {
            Some(s) => s,
            None => {
                log::error!("Failed to prepare query!");
                return None;
            }
        };
        statement
            .bind((1, cloned_udid.to_string().as_str()))
            .unwrap();
        if let Some(State::Row) = crate::db::statement_next(&mut statement) {
            let ip = statement.read::<String, _>("ip").unwrap();
            info!("Found device with udid {} already in db", cloned_udid);

            // Delete the device from the database
            let query = "DELETE FROM devices WHERE udid = ?";
            let mut statement = match crate::db::db_prepare(&db, query) {
                Some(s) => s,
                None => {
                    log::error!("Failed to prepare query!");
                    return None;
                }
            };
            statement
                .bind((1, cloned_udid.to_string().as_str()))
                .unwrap();
            if crate::db::statement_next(&mut statement).is_none() {
                log::error!("Failed to enact the statement");
            }

            Some(ip)
        } else {
            None
        }
    })
    .await
    {
        Ok(ip) => ip,
        Err(e) => {
            info!("Failed to get IP from database: {:?}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to get IP"));
        }
    };

    let register_mode = std::env::var("ALLOW_REGISTRATION")
        .unwrap_or("1".to_string())
        .parse::<u8>()
        .unwrap();

    let client_config: Vec<u8>;
    let ip_final: Ipv6Addr;

    if register_mode == 1 {
        // register using wireguard
        let wireguard_config_name =
            std::env::var("WIREGUARD_CONFIG_NAME").unwrap_or("jitstreamer".to_string());
        let wireguard_conf = format!("/etc/wireguard/{wireguard_config_name}.conf");
        let wireguard_port = std::env::var("WIREGUARD_PORT")
            .unwrap_or("51869".to_string())
            .parse::<u16>()
            .unwrap_or(51869);
        let wireguard_server_address =
            std::env::var("WIREGUARD_SERVER_ADDRESS").unwrap_or("fd00::/128".to_string());
        let wireguard_endpoint =
            std::env::var("WIREGUARD_ENDPOINT").unwrap_or("jitstreamer.jkcoxson.com".to_string());
        let wireguard_server_allowed_ips =
            std::env::var("WIREGUARD_SERVER_ALLOWED_IPS").unwrap_or("fd00::/64".to_string());

        // Read the Wireguard config file
        let mut server_peer = match wg_config::WgConf::open(&wireguard_conf) {
            Ok(conf) => conf,
            Err(e) => {
                info!("Failed to open Wireguard config: {:?}", e);
                if let wg_config::WgConfError::NotFound(_) = e {
                    // Generate a new one

                    let key =
                        wg_config::WgKey::generate_private_key().expect("failed to generate key");
                    let interface = wg_config::WgInterface::new(
                        key,
                        wireguard_server_address.parse().unwrap(),
                        Some(wireguard_port),
                        None,
                        None,
                        None,
                    )
                    .unwrap();

                    wg_config::WgConf::create(wireguard_conf.as_str(), interface, None)
                        .expect("failed to create config");

                    info!("Created new Wireguard config");

                    wg_config::WgConf::open(wireguard_conf.as_str()).unwrap()
                } else {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "failed to open server Wireguard config",
                    ));
                }
            }
        };
        let mut public_ip = None;
        if let Some(ip) = ip {
            match server_peer.peers() {
                Ok(peers) => {
                    for peer in peers {
                        let peer_ip = peer.allowed_ips();
                        if ip.is_empty() {
                            continue;
                        }
                        if peer_ip[0].to_string() == ip {
                            info!("Found peer with IP {}", ip);

                            public_ip = Some(peer.public_key().to_owned());
                        }
                    }
                }
                Err(e) => {
                    info!("Failed to get peers: {:?}", e);
                    return Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to get peers"));
                }
            }
        }

        if let Some(public_ip) = public_ip {
            server_peer = server_peer.remove_peer_by_pub_key(&public_ip).unwrap();
        }

        let ip = generate_ipv6_from_udid(udid.as_str());
        ip_final = ip;
        // Generate a new peer for the device
        client_config = match server_peer.generate_peer(
            std::net::IpAddr::V6(ip),
            wireguard_endpoint.parse().unwrap(),
            vec![wireguard_server_allowed_ips.parse().unwrap()],
            None,
            true,
            Some(20),
        ) {
            Ok(config) => config.to_string().as_bytes().to_vec(),
            Err(e) => {
                info!("Failed to generate peer: {:?}", e);
                return Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to generate peer"));
            }
        };
    } else if register_mode == 2 {
        // register directly using request IP
        ip_final = match client_ip.0 {
            IpAddr::V4(v4) => v4.to_ipv6_mapped(),
            IpAddr::V6(v6) => v6,
        };
        client_config = ip_final.to_string().as_bytes().to_vec();
    } else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unknown registration mode",
        ));
    }

    // Save the plist to the storage
    let plist_storage_path = std::env::var("PLIST_STORAGE").unwrap_or(
        match std::env::consts::OS {
            "macos" => "/var/db/lockdown",
            "linux" => "/var/lib/lockdown",
            "windows" => "C:/ProgramData/Apple/Lockdown",
            _ => panic!("Unsupported OS, specify a path"),
        }
        .to_string(),
    );

    // Create the folder if it doesn't exist
    if let Err(e) = tokio::fs::create_dir_all(&plist_storage_path).await {
        log::error!("Failed to create plist storage path: {e:?}");
    }

    tokio::fs::write(
        format!("{plist_storage_path}/{udid}.plist"),
        &plist_bytes.to_vec(),
    )
    .await
    .map_err(|e| {
        info!("Failed to save plist: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to save plist")
    })?;

    // Save the IP to the database
    tokio::task::spawn_blocking(move || {
        let db = match sqlite::open("jitstreamer.db") {
            Ok(db) => db,
            Err(e) => {
                info!("Failed to open database: {:?}", e);
                return;
            }
        };

        // Insert the device into the database
        let query = "INSERT INTO devices (udid, ip, last_used) VALUES (?, ?, CURRENT_TIMESTAMP)";
        let mut statement = match crate::db::db_prepare(&db, query) {
            Some(s) => s,
            None => {
                log::error!("Failed to prepare query!");
                return;
            }
        };
        statement
            .bind(&[(1, udid.as_str()), (2, ip_final.to_string().as_str())][..])
            .unwrap();
        if crate::db::statement_next(&mut statement).is_none() {
            log::error!("Failed to enact the statement");
        }
    });

    if register_mode == 1 {
        refresh_wireguard();
    }

    Ok(client_config.into())
}

const UPLOAD_HTML: &str = include_str!("../src/upload.html");

pub async fn upload() -> Result<Html<&'static str>, (StatusCode, &'static str)> {
    Ok(Html(UPLOAD_HTML))
}

fn generate_ipv6_from_udid(udid: &str) -> std::net::Ipv6Addr {
    // Hash the UDID using SHA-256
    let mut hasher = sha2::Sha256::new();
    hasher.update(udid.as_bytes());
    let hash = hasher.finalize();

    // Use the first 64 bits of the hash for the interface ID
    let interface_id = u64::from_be_bytes(hash[0..8].try_into().unwrap());

    // Set the first 64 bits to the `fd00::/8` range (locally assigned address)
    let mut segments = [0u16; 8];
    segments[0] = 0xfd00; // First segment in the `fd00::/8` range
    (1..8).for_each(|i| {
        let shift = (7 - i) * 16;
        segments[i] = if shift < 64 {
            ((interface_id >> shift) & 0xFFFF) as u16
        } else {
            0
        };
    });

    std::net::Ipv6Addr::from(segments)
}

fn refresh_wireguard() {
    let wireguard_config_name =
        std::env::var("WIREGUARD_CONFIG_NAME").unwrap_or("jitstreamer".to_string());

    // wg syncconf jitstreamer <(wg-quick strip jitstreamer)
    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(format!(
            "wg syncconf jitstreamer <(wg-quick strip {wireguard_config_name})"
        ))
        .output()
        .expect("failed to execute process");

    info!("Refreshing Wireguard: {:?}", output);
}
