// Jackson Coxson

use reqwest::blocking::get;
use std::fs;
use std::path::Path;

const URLS: [&str; 3] = ["https://github.com/doronz88/DeveloperDiskImage/raw/refs/heads/main/PersonalizedImages/Xcode_iOS_DDI_Personalized/BuildManifest.plist", "https://github.com/doronz88/DeveloperDiskImage/raw/refs/heads/main/PersonalizedImages/Xcode_iOS_DDI_Personalized/Image.dmg", "https://github.com/doronz88/DeveloperDiskImage/raw/refs/heads/main/PersonalizedImages/Xcode_iOS_DDI_Personalized/Image.dmg.trustcache"];
const OUTPUT_DIR: &str = "DDI";
const OUTPUT_FILES: [&str; 3] = [
    "DDI/BuildManifest.plist",
    "DDI/Image.dmg",
    "DDI/Image.dmg.trustcache",
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Ensure output directory exists
    if !Path::new(OUTPUT_DIR).exists() {
        fs::create_dir_all(OUTPUT_DIR).expect("Failed to create DDI directory");
    }

    // Check if the file already exists
    if Path::new(OUTPUT_FILES[0]).exists() {
        return;
    }

    // Download the file using reqwest
    println!("Downloading BuildManifest.plist...");
    for (i, url) in URLS.iter().enumerate() {
        let response = get(*url).expect("Failed to send request");
        let bytes = response.bytes().expect("Failed to read response");
        fs::write(OUTPUT_FILES[i], &bytes).expect("Failed to write file");
    }
}
