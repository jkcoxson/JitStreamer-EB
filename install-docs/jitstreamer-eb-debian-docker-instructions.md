# How to Self-Host JITStreamer-EB (For Dummies)

## What is this?

I write these little guides for myself and sometimes like to share. My goal is only to provide more precise and full instructions to those who have very little to no knowledge of the required tools.

This guide will be going over how to get JITStreamer-EB (I'll refer to it as the "project" in the future to shorten my typing) up and running in Docker. Specifically using Docker Compose. Docker Compose offers, in my opinion, a much easier deployment of Docker containers. This is absolutely not the only way to run this project.

## Credit and Sources

The GOAT jkcoxson created this beautiful project for us. Here's his [GitHub](https://github.com/jkcoxson) and here's the specific [project](https://github.com/jkcoxson/JitStreamer-EB) of interest. Also its [website](https://jkcoxson.com/jitstreamer). He provided this to everyone for free. You should consider donating to him to show appreciation and support this type of work in the future.

[Docker](https://www.docker.com/) of course

[SideStore](https://docs.sidestore.io) I used one of their documents to cut down on me re-typing. Another excellent project worth checking out separately.

[jkcoxson's Discord](https://discord.gg/cRDk9PN9zu) Small but helpful and nice community. They put together and troubleshot issues with self-hosting and getting everything to work well with Docker Compose within days of jkcoxson going live with the project. It was beautiful to witness and take a small part in.

Here's some other sites and specific links I found personally useful for random "How do I do..." questions:

[Sqlite Tutorial](https://www.sqlitetutorial.net/)

[Creating and Deleting Databases and Tables](https://www.prisma.io/dataguide/sqlite/creating-and-deleting-databases-and-tables)

[Inserting and Deleting Data](https://www.prismagraphql.com/dataguide/sqlite/inserting-and-deleting-data)

[UFW in Debian](https://wiki.debian.org/Uncomplicated%20Firewall%20%28ufw%29)

## Prerequisites

Before beginning, you'll need to "gather your tools" and "parts" required to build the end project.

I will be writing this guide with an eye towards installing things on Linux. Specifically, I'm currently running a derivative of Debian called "MX Linux". I tell you this because no guide can be universal. However, Docker offers something closer to universality across operating systems. Just be aware that the way directories work and such are different. Permissions are handled differently. Not everything will always work the same way.

### Install [Docker](https://docs.docker.com/engine/install/debian/) with the Docker Compose plugin.

Go to the hyperlink (Debian users) or otherwise locate your required version of Docker. Here's the instructions for Debian (if you're using another OS, just use your relevant instructions then move on).

**This is me assuming you are installing Docker on a fresh OS** (as I am too!). Go to the link and follow instructions if you need to "clean out" old stuff. (You probably do not need to).

1. Add the Docker sources

```
# Add Docker's official GPG key:
sudo apt-get update
sudo apt-get install ca-certificates curl
sudo install -m 0755 -d /etc/apt/keyrings
sudo curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc
sudo chmod a+r /etc/apt/keyrings/docker.asc

# Add the repository to Apt sources:
echo \
  "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian \
  $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
  sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
sudo apt-get update
```

2. Install

```sudo apt-get install docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin```

3. Test

```sudo docker run hello-world```

The above step should pop out a message beginning with "Hello from Docker!" and a long message after that. If so, good, Docker is installed. We're going to add our user to the docker group so that we don't need to type "sudo docker" constantly. You can skip this if you don't care about typing sudo and your password constantly.

**Note: I feel compelled to mention that adding your user to the "docker" group is considered a possible security risk. I won't comment further, but I will leave the [link](https://docs.docker.com/engine/security/#docker-daemon-attack-surface) from the Docker website directly. This was brought to my attention by user "@Laes" in the Discord (Thank you for informing me.) I won't delete the information below, but I will say if you go to the link and read it and don't fully comprehend what it says, then maybe just stick to typing ```sudo docker```. As Laes put it, (paraphrasing) "You only have to type ```sudo docker``` a few times then never again." Which is true, and I agree with this sentiment now.**

Copy/pasted from [here](https://docs.docker.com/engine/install/linux-postinstall/). Go there for more details. It's pretty straight forward, though.

```sudo groupadd docker```

```sudo usermod -aG docker $USER```

```newgrp docker```

```docker run hello-world```

You should once again get the "Hello from Docker" message. Yay. Docker is ready to dock... stuff?

## Part I - Preparation

With Docker set up, you need to create some more files, place them in specific locations, and download some stuff. On we go.

### Clone the Repo

**Note: I (unlearned6688) have begun pushing Docker image builds to my hub.docker.com account sardine0006. If you'd like to skip building the image yourself, you can pull the image manually ```docker image pull sardine0006/jitstreamer-eb:latest``` or just edit your docker-compose.yml and make the "image:" portion say ```sardine0006/jitstreamer-eb:latest``` If you do grab tbe pre-built image, you can also skip the build image portion down in the "Exeuction" section. Otherwise, if you prefer to build yourself, continue on!**

Again, this is all from a Debian perspective. Adjust commands as required. Google will help you. Many of these are now the same using tools like Powershell on Windows. MacOS is Unix based and thus already basically the same for most stuff.

I will be cloning the repo into my "home" directory. It will look like ```/home/USER/JitStreamer-EB``` where USER is... you know. Your username that you login with. You can also shortcut to it with ```~/JitStreamer-EB```. Just know the directory where you will be building.

Clone the repo to your local server/PC. Change directory to the newly cloned repo.

```
git clone https://github.com/jkcoxson/JitStreamer-EB
cd JitStreamer-EB
```

### Create a Pairing File

You can create a pairing file many ways. You can create them in any OS and copy them to this docker container. So, if you find it easier to make them in Windows, then do so. Whatever makes you happy.

I will be copying the [instructions](https://docs.sidestore.io/docs/getting-started/pairing-file/) from the SideStore project on how to get a pairing file with Linux. They have instructions for Windows and MacOS as well. Follow the link for downloads and more instructions.

1. Extract the Jitterbug zip file, and open a terminal (if you haven't already) to the extracted directory.
2. In that terminal, run ```chmod +x ./jitterbugpair```
3. Plug your device into your computer, and open your device to its home screen. Once done, execute the program in your terminal with ```./jitterbugpair```
4. If you get a prompt saying you need to trust the computer from your iDevice, make sure to do so. You may need to rerun jitterbugpair.
5. Once it is done, you will get a file that ends with .mobiledevicepairing in the directory you ran jitterbugpair from.
6. Transfer this file to your device in a way of your choosing. Zipping the file before sending it off is the best way to ensure the pairing file won't break during transport
7. Transferring using cloud storage may change the file's extension (most likely turning into a .txt file), so be careful. It is also possible to change the extension to .plist for use with older SideStore versions, like 0.1.1.

Note: You **DO** need to change the extension to .plist. Change it now!

Note 2: Make a pairing file for each iDevice you want to user JitStreamer-EB for. iPhone and iPad means you need two different pairing files.

Note 3: (Holy notes) You will need your UDID for each device later as well. These pairing files have their UDID in the names! Wow! So make sure to note which UDID belongs to each iDevice you own. This will save you brain-pain later.

### Copy Pairing Files (.plist) to JitStreamer-EB

You can do this via GUI (with a file manager) or in the terminal. Whatever brings you happiness and joy. GUI is probably easiest for most people. In which case just copy the file(s), go to ```~/JitStreamer-EB``` and paste it in the "lockdown" directory. Otherwise, here's terminal instructions:

1. In terminal, go to whatever directory you ran jitterbug in when you created your pairing file. The default is the Downloads directory  

```cd ~/Downloads```

2. Make the lockdown directory for the plist files. (If you ran the container already for some reason, this directory already exists.)

```mkdir ~/JitStreamer-EB/lockdown```

3. Type "ls" first to list all the files. It makes copy/paste easier with these huge file names.

```ls```

4. Note: The below file name is FAKE. You need to use your own file with your own device's UDID. The ls command will show all your file names. Just copy the relevant ones.

```cp 00008111-111122223333801E.plist ~/JitStreamer-EB/lockdown```

Easy!

### Create Your Database File

This part might be a bit weird. Prepare yourself.

1. You have to create the jitstreamer.db file (sqlite database) using build instructions included in the repo.

```
mkdir app
sqlite3 ./jitstreamer.db < ./src/sql/up.sql
```

2. Now you have a fancy little database. But you need to add your device info into it.

Note: There are likely many ways to achieve the desired result here. This is just how I did it. It may be more steps than required, but it lets you see how the database works internally which I prefer for myself.

Type into the terminal

```sqlite3```

Something like this will appear (maybe different versions):

```
SQLite version 3.40.1 2022-12-28 14:03:47
Enter ".help" for usage hints.
Connected to a transient in-memory database.
Use ".open FILENAME" to reopen on a persistent database.
```

Note that it says "transient in-memory database". We don't want that!
It's an easy fix.

Type into the terminal after the "sqlite>" (which you should see):

```.open jitstreamer.db```

Optional: "Is everything ok so far?" check. Type:

```.tables```

You should see:

```devices       downloads     launch_queue```

This means your tables inside the database were created as instructed above. Good, good.

3. **Read the entire next part, including the notes and everything. Don't just blindly copy/paste! You must edit stuff!**

Type this command from the repository to add your device information to the DEVICES table.

```INSERT INTO DEVICES (udid, ip, last_used) VALUES ([udid], [ip], CURRENT_TIMESTAMP);```

Replace the [udid] and [ip] (so, the second set. The two with the brackets!) with (examples)'00008111-111122223333801E' and '192.168.1.2'

Note 1: The above UDID is FAKE. INSERT YOUR OWN UDID! I used a fake one which resembles a real one to help visually. Please... don't copy that into your database.

Note 2: The brackets are now deleted. They are replaced with ' (NOT ")

Note 3: The IP in question is the IP of the iDevice. The IP of your iPhone, etc. Each device needs to have a different IP.

Note 4: You **HAVE** to add "::ffff:" in front of the regular IPv4 IP address. eg: ```::ffff:192.168.1.2```

This is a FAKE but realistic example. Yours will contain your own UDID and IP.

```INSERT INTO DEVICES (udid, ip, last_used) VALUES ('00008111-111122223333801E', '::ffff:192.168.1.2', CURRENT_TIMESTAMP);```

Now you can quickly check "Did I do this correctly?" (The casing is important here)

**Note: That semicolon on the end (;) is REQUIRED for this command to work correctly! We love our ; Don't drop it**

```SELECT * FROM devices;```

You should see something like this for each device you inserted above.

```::ffff:192.168.1.2|00008111-111122223333801E|2025-01-31 15:17:50```

If you got the above response, then you are done with database creation. 

4. Press "Ctrl Key + D" (two keys) to exit from the sqlite screen.

## Part II - The Execution

We got everything created and placed where it needs to be. We're ready to do some JITing.

### Build and Run the Docker Image

**NOTE: Step 1 below (building the image) is NO LONGER NECESSARY! If you wish to build the image, then by all means, feel free. However, if you're docker-compose.yml file has ```image: jkcoxson/jitstreamer-eb:latest``` then you can skip to Step 2 below. Your Docker image will be automatically pulled and run once it's downloaded!**

1. Build the docker image! This takes a bit to download then compile. Ensure it finishes with no errors. 

Note: you are still in ```~/JitStreamer-EB``` when you run this!

```docker build -t jitstreamer-eb .```

2. Create a docker container using the Docker image you just made. Note: you are still in ```~/JitStreamer-EB``` when you run this!

```docker compose up -d```

3. I like to see the logs running live with:

```docker logs -t -f jitstreamer-eb```

You should see a long stream of stuff happening. Good, good. (Ctrl + C will take you out of this screen)

### Setting Up the Shortcut on Your iDevice

1. On your iPhone or iPad, go to this [site](https://jkcoxson.com/jitstreamer)

2. Go to the bottom. Download that Shortcut. You will also need the Shortcuts app for iOS, obviously. Download it if you need to from the Apple App Store.

3. Open Shortcuts on iOS

4. Locate the JitStreamer EB shortcut in the list

5. Long press on it

6. Select the option "Edit"

7. Locate the IP section that needs changed. Under the long introduction from jkcoxson, you'll see a section with a yellow-colored icon called "Text". In the editable area it will have http://[an-ip]:9172 . This is the area we will be changing.

8. Change it so that it matches the IP of your HOST machine. The machine you are running Docker on. Example:

```http://192.168.1.3:9172```

Obviously the IP would be your own IP.

9. Hit "Done" in the upper-right corner.

### Oh Yeah, It's All Coming Together... Time to JIT

Pre-flight checklist

1. Your docker container is running on your host machine without any crazy errors. Type ```docker ps``` to see a list of running containers.

2. Your host machine (where the Docker container is running) is on the same network as your iDevice. You can change this later, but for testing purposes at first, be on the same network to eliminate that as a possible source of problems.

3. Your edited the Shortcut from jkcoxson's website as instructed.

Good?

Tap the Shortcut to run it!

If you have the Docker container logs running still (I recommend you do!) you will see some stuff happening as your iDevice connects to your host and docker container. Do not leave the Shortcuts app. Just wait a moment. It takes a couple seconds. If everything is going ok, it will pop up with a list of apps. You will select the app you want to enable JIT for. The shortcut and container will begin working again. Give it another moment to finish. Eventually it will say something like "You are 0 in the queue." After which (give it another moment!) your app should launch automatically. 

If that happens, congratulations. You are JITing.

## Troubleshooting and (Short) FAQ

This section will be for the future mostly. It's hard for me to predict or know what issues people will encounter. I tried to explain to the best of my ability, but it's possible I forgot things or overlooked things.

### Firewall and Network Issues

On the general topic of "Network issues" it's hard to provide much advice because every issue is specific. If you are having issue with connecting and acquiring JIT with your iDevice, and you already did some googling around online and reading, then my best recommendation is join the discord and search there. Ask if search doesn't provide much help.

On firewall stuff I can offer more information in specific regard to Debian. Debian ships with (to my knowledge, this is standard install) a firewall called "Uncomplicated FireWall" or "UFW". You can access it with the command ```sudo ufw --help```

I bring up ufw because I was unable to connect my iDevice to my Debian host (and then to the Docker container) until I allowed my iDevice IP through UFW. Example:

```sudo ufw allow from 192.168.1.2```

```sudo ufw allow to 192.168.1.2```

Then it worked perfectly. It may also work if you allow the port 9172, but I haven't tried. The above worked. Good enough for me.

**Update on the above point: ```sudo ufw allow 9172``` DOES WORk**

### Network_bridge mode with Docker compose?

**YES** It does work. Or, it should, anyway.
Here's my working docker-compose.yml to use network bridge mode. At some point this will probably get added to the repo separately.

```
services:
    jitstreamer-eb:
        container_name: jitstreamer-eb
        #network_mode: host
        volumes:
            - ./lockdown:/var/lib/lockdown
           # - ./wireguard:/etc/wireguard
            - ./jitstreamer.db:/app/jitstreamer.db
        environment:
            - RUST_LOG=info
            - RUNNER_COUNT=1
            - ALLOW_REGISTRATION=2
        ports:
            - 9172:9172
        cap_add:
            - NET_ADMIN
        devices:
            - /dev/net/tun:/dev/net/tun
        image: jkcoxson/jitstreamer-eb:latest
        restart: unless-stopped
```
Note that this docker-compose file will: run in network bridge mode, not create a wireguard directory or .conf file, and exposes on port 9172.

### What does this work with?

I saw a lot of discussion day one about what this (and all known JIT methods, to my knowledge) works with.

TL;DR: **Only sideloaded apps using a developer certificate.**

(Free or paid. Free is still arbitrarily limited by Apple to only 3 active apps, 10 apps per week total. That is an Apple limitation. Please compalain to them if you also find it annoying.)

"But I purchased a cert from xxxx and I can't see my apps! Why?!"

It isn't a developer cert. It's just a distribution cert. You have to contact the seller of that cert if you want to change it to a developer cert. This has nothing to do with this project. It's an Apple limitation, ultimately.

You can also purchase your own Apple Developer Account ($99/year) and sideload using that.

Or use the free developer account that every Apple Account has. You could sideload using something like SideStore, AltStore or (I haven't personally tried this one for JIT- mileage may vary) Sideloadly. This uses your personal, free developer certificate and thus allows you to access JIT on the sideloaded apps.

My personal setup is:

- a cheap distribution cert used for non-JIT apps.

- SideStore for apps I want JIT for.

But do whatever makes you happy. Just remember it's on Apple for any and all of these limitations. Or why we must use special tools to acquire JIT in the first place.
