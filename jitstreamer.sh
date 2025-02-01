#!/bin/bash

# ASCII Art Header
cat << "EOF"
       ___ __  _____ __                                            __________ 
      / (_) /_/ ___// /_________  ____ _____ ___  ___  _____      / ____/ __ )
 __  / / / __/\__ \/ __/ ___/ _ \/ __ `/ __ `__ \/ _ \/ ___/_____/ __/ / __  |
/ /_/ / / /_ ___/ / /_/ /  /  __/ /_/ / / / / / /  __/ /  /_____/ /___/ /_/ / 
\____/_/\__//____/\__/_/   \___/\__,_/_/ /_/ /_/\___/_/        /_____/_____/  

CREATED BY michaell._.

EOF

sleep 2
# Check if running on Debian or Ubuntu
if ! grep -qi 'ubuntu\|debian' /etc/os-release; then
    echo "This script only works on debian-based systems. Exiting..."
    exit 1
fi

# Uninstall conflicting packages
echo "Uninstalling any conflicting Docker packages..."
for pkg in docker.io docker-doc docker-compose podman-docker containerd runc; do
    sudo apt-get remove -y $pkg
done

# Function to set up Docker's apt repository for Debian
setup_debian_docker_repo() {
    sudo apt-get update
    sudo apt-get install -y ca-certificates curl
    sudo install -m 0755 -d /etc/apt/keyrings
    sudo curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc
    sudo chmod a+r /etc/apt/keyrings/docker.asc

    echo \
      "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian \
      $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
      sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
}

# Function to set up Docker's apt repository for Ubuntu
setup_ubuntu_docker_repo() {
    echo "Setting up Docker's apt repository for Ubuntu..."
    sudo apt-get update
    sudo apt-get install -y ca-certificates curl
    sudo install -m 0755 -d /etc/apt/keyrings
    sudo curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
    sudo chmod a+r /etc/apt/keyrings/docker.asc
    echo \
      "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu \
      $(. /etc/os-release && echo "${UBUNTU_CODENAME:-$VERSION_CODENAME}") stable" | \
      sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
}

# Check if Debian or Ubuntu and set repository accordingly
if grep -qi 'ubuntu' /etc/os-release; then
    setup_ubuntu_docker_repo
elif grep -qi 'debian' /etc/os-release; then
    setup_debian_docker_repo
fi

# Update package index again
sudo apt-get update

# Install Docker packages
echo "Installing Docker..."
sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

# Enable Docker and containerd to start on boot
echo "Enabling Docker and containerd to start on boot..."
sudo systemctl enable docker.service
sudo systemctl enable containerd.service



# Set permissions for 'jitstreamer-eb' folder
# Set permissions for 'jitstreamer-eb' folder
JIT_DIR="${HOME}/JitStreamer-EB"
if [ ! -d "$JIT_DIR" ]; then
    echo "JitStreamer-EB directory not found. Creating it and setting permissions..."
    mkdir -p "$JIT_DIR"
    sudo chown $(whoami):$(whoami) "$JIT_DIR"
    sudo chmod 755 "$JIT_DIR"
else
    echo "JitStreamer-EB directory exists. Setting permissions..."
    sudo chown $(whoami):$(whoami) "$JIT_DIR"
    sudo chmod 755 "$JIT_DIR"
fi

# Debugging: Print the directory
echo "JitStreamer-EB directory path: $JIT_DIR"


# Check if JitStreamer-EB directory exists, if not, clone it
if [ ! -d "$JIT_DIR" ]; then
    echo "JitStreamer-EB directory not found in home directory. Cloning from GitHub..."
    git clone https://github.com/jkcoxson/JitStreamer-EB.git "$JIT_DIR" || { echo "Failed to clone repository. Exiting..."; exit 1; }
else
    echo "JitStreamer-EB directory already exists. Checking for updates..."
    cd "$JIT_DIR" || { echo "Failed to change directory to $JIT_DIR. Exiting..."; exit 1; }
    
    # Check if the directory is a Git repository
    if [ ! -d ".git" ]; then
        echo "The JitStreamer-EB directory is not a valid Git repository."
        echo "Would you like to remove the existing directory and clone it again? (y/n)"
        read -r response
        if [[ "$response" =~ ^[Yy]$ ]]; then
            rm -rf "$JIT_DIR" || { echo "Failed to remove existing directory. Exiting..."; exit 1; }
            git clone https://github.com/jkcoxson/JitStreamer-EB.git "$JIT_DIR" || { echo "Failed to clone repository. Exiting..."; exit 1; }
        else
            echo "Exiting without making changes."
            exit 1
        fi
    else
        echo "Updating the existing repository..."
        git pull || { echo "Failed to update repository. Exiting..."; exit 1; }
    fi
fi

cd "$JIT_DIR" || { echo "Failed to change directory to $JIT_DIR. Exiting..."; exit 1; }

# Install dependencies
echo "Installing dependencies..."
sudo apt update && sudo apt install -y usbmuxd sqlite3 unzip libimobiledevice6 libimobiledevice-utils

# Create necessary directories
echo "Creating necessary directories..."
mkdir -p lockdown
mkdir -p wireguard

# Check if database exists and has tables
if [ ! -f ./jitstreamer.db ]; then
    echo "Creating new database..."
    sqlite3 ./jitstreamer.db < ./src/sql/up.sql
else
    echo "Database already exists, checking tables..."
    # Check if devices table exists
    TABLE_EXISTS=$(sqlite3 ./jitstreamer.db "SELECT name FROM sqlite_master WHERE type='table' AND name='devices';")
    if [ -z "$TABLE_EXISTS" ]; then
        echo "Tables not found, initializing database..."
        sqlite3 ./jitstreamer.db < ./src/sql/up.sql
    fi
fi

# Ask user for the pairing file
echo "Please enter the full path to your pairing file or place it in this directory:"
echo "$PWD"
read -r pairing_path
if [ -f "$pairing_path" ]; then
    # Extract filename from path
    filename=$(basename "$pairing_path")
    # Create plist filename
    plist_filename="${filename%.*}.plist"
    # Copy and rename
    cp "$pairing_path" "./lockdown/$plist_filename"
    echo "Copied pairing file to lockdown directory as $plist_filename"
else
    echo "Pairing file not found at the specified location. Exiting..."
    exit 1
fi

# Get UDID from the plist file name
UDID=$(basename -s .plist lockdown/*.plist)

# Check if UDID already exists in database
EXISTING_UDID=$(sqlite3 jitstreamer.db "SELECT udid FROM devices WHERE udid='$UDID';")

if [ ! -z "$EXISTING_UDID" ]; then
    echo "Device with UDID $UDID already exists in database. Skipping insertion."
else
    # Ask for device IP
    echo "Please enter your device's IP address (e.g., 192.168.1.2):"
    read -r device_ip

    # Add ::ffff: prefix to the IP address
    ipv6_ip="::ffff:$device_ip"

    # Create SQL command
    SQL_COMMAND="INSERT INTO devices (udid, ip, last_used) VALUES ('$UDID', '$ipv6_ip', CURRENT_TIMESTAMP);"

    # Execute SQL command
    echo "Adding device to database..."
    sqlite3 jitstreamer.db "$SQL_COMMAND"

    # Verify the entry was added
    echo "Verifying database entry..."
    sqlite3 jitstreamer.db "SELECT * FROM devices;"
fi

# Build the Docker image
echo "Building the Docker image..."
sudo docker build -t jitstreamer-eb .

# Run the Docker container using Docker Compose
echo "Running the Docker container..."
sudo docker compose up -d

: '
# Create jitstreamer.sh in the home directory
JITSTREAMER_SCRIPT="$HOME/jitstreamer.sh"
cat << 'EOF' > "$JITSTREAMER_SCRIPT"
#!/bin/bash

# Name of the Docker container
CONTAINER_NAME="jitstreamer-eb"
JIT_DIR="${HOME}/JitStreamer-EB"  # Path to the JitStreamer-EB directory

# Function to start the container
start_container() {
    echo "Starting the $CONTAINER_NAME container..."
    cd "$JIT_DIR" || { echo "Failed to change directory to $JIT_DIR. Exiting..."; exit 1; }
    sudo docker compose start "$CONTAINER_NAME"
}

# Function to stop the container
stop_container() {
    echo "Stopping the $CONTAINER_NAME container..."
    cd "$JIT_DIR" || { echo "Failed to change directory to $JIT_DIR. Exiting..."; exit 1; }
    sudo docker compose stop "$CONTAINER_NAME"
}

# Function to restart the container
restart_container() {
    echo "Restarting the $CONTAINER_NAME container..."
    cd "$JIT_DIR" || { echo "Failed to change directory to $JIT_DIR. Exiting..."; exit 1; }
    sudo docker compose restart "$CONTAINER_NAME"
}

# Check if at least one argument is provided
if [ "$#" -eq 0 ]; then
    echo "No command provided. Use start, stop, restart, or logs."
    exit 1s
fi

# Handle the command
case "$1" in
    start)
        start_container
        ;;
    stop)
        stop_container
        ;;
    restart)
        restart_container
        ;;

    *)
        echo "Invalid command. Use start, stop, restart."
        exit 1
        ;;
esac
EOF
'
# Make jitstreamer.sh executable
#chmod +x "$JITSTREAMER_SCRIPT"

# Install the script itself to /usr/local/bin
#SCRIPT_NAME="jitstreamereb"
#INSTALL_PATH="/usr/local/bin/$SCRIPT_NAME"

#if [ "$(id -u)" -ne 0 ]; then
#    echo "This script needs to be run with sudo to install itself."
#    exit 1
#fi

# Copy the script to /usr/local/bin
#sudo cp "$JITSTREAMER_SCRIPT" "$INSTALL_PATH"
#sudo chmod +x "$INSTALL_PATH"
#echo "Script installed as $SCRIPT_NAME. You can now run it from anywhere."

# Show usage examples and instructions
#echo -e "\nUsage examples for the jitstreamereb command:"
#echo "1. jitstreamereb start   # To start the container"
#echo "2. jitstreamereb stop    # To stop the container"
#echo "3. jitstreamereb restart  # To restart the container"

# Get the current IP address
IP_ADDRESS=$(hostname -I | awk '{print $1}')
echo -e "\nPlease change the address in your shortcut to the following format:"
echo "http://$IP_ADDRESS:9172"
