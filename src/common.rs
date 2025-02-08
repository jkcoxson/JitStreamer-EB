// Jackson Coxson

use idevice::pairing_file::PairingFile;
use log::info;

pub async fn get_udid_from_ip(ip: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let db = match sqlite::open("jitstreamer.db") {
            Ok(db) => db,
            Err(e) => {
                info!("Failed to open database: {:?}", e);
                return Err(format!("Failed to open database: {:?}", e));
            }
        };

        // Get the device from the database
        let query = "SELECT udid FROM devices WHERE ip = ?";
        let mut statement = match crate::db::db_prepare(&db, query) {
            Some(s) => s,
            None => {
                log::error!("Failed to prepare query!");
                return Err("Failed to open database".to_string());
            }
        };
        statement.bind((1, ip.as_str())).unwrap();
        let udid = if let Some(sqlite::State::Row) = crate::db::statement_next(&mut statement) {
            let udid = statement.read::<String, _>("udid").unwrap();
            info!("Found device with udid {}", udid);
            udid
        } else {
            info!("No device found for IP {:?}", ip);
            return Err(format!("No device found for IP {:?}", ip));
        };
        Ok(udid)
    })
    .await
    .unwrap()
}

/// Gets the pairing file
pub async fn get_pairing_file(udid: &str) -> Result<PairingFile, idevice::IdeviceError> {
    // All pairing files are stored at /var/lib/lockdown/<udid>.plist
    let path = format!("/var/lib/lockdown/{}.plist", udid);
    let pairing_file = tokio::fs::read(path).await?;

    PairingFile::from_bytes(&pairing_file)
}
