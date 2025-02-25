# Installing JITStreamer on raspberry pi (works offline, sort of)

## Supported devices

Tested:
* Raspberry PI 4 B with iPad type C

Other devices might work but I don't know yet. (Contribute if you figure something out)

## Step 1 - Enable OTG 

By following tutorials such as [this](https://blog.hardill.me.uk/2019/11/02/pi4-usb-c-gadget/), you should be able to enable OTG on your device. OTG means the type C port on the pi (4) will act as a power and data transfer (an ethernet port). This tutorial is slightly outdated and not exactly what we need so I would recommend using [this repo's](https://github.com/techcraftco/rpi-usb-gadget) prebuilt image. Specifically, [this](https://github.com/techcraftco/rpi-usb-gadget/releases/download/v0.4/ubuntu-server-arm64-22.04-arm64.img.zip) ubuntu server image.



## Step 2 - Basic Setup
SSH into the pi
# Update
Run the update commands 
``` 
sudo apt update
sudo apt upgrade
```
When prompted to change dns settings, type 'N' and press enter. I will keep the existing files.

# Docker
Install by following [this tutorial](https://docs.docker.com/engine/install/ubuntu/#install-using-the-repository) from the official docker website.

## JITStreamer

Follow the normal step from [here](https://github.com/jkcoxson/JitStreamer-EB/blob/master/install-docs/jitstreamer-eb-debian-docker-instructions.md#part-i---preparation), and stop when you reach [here](https://github.com/jkcoxson/JitStreamer-EB/blob/master/install-docs/jitstreamer-eb-debian-docker-instructions.md#create-your-database-file)

Here is a modified version of the step for raspberry pi

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

If sqlite3 is not installed, run
```sudo apt install sqlite3```

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

```devices       downloads     launch_queue  mount_queue```

This means your tables inside the database were created as instructed above. Good, good.
**Edit the following commands with YOUR udid**
Type each command one by one
```INSERT INTO DEVICES (udid, ip, last_used) VALUES ('YOUR UDID', '::ffff:10.55.0.2', CURRENT_TIMESTAMP);```
```INSERT INTO DEVICES (udid, ip, last_used) VALUES ('YOUR UDID', '::ffff:10.55.0.3', CURRENT_TIMESTAMP);```
```INSERT INTO DEVICES (udid, ip, last_used) VALUES ('YOUR UDID', '::ffff:10.55.0.4', CURRENT_TIMESTAMP);```
```INSERT INTO DEVICES (udid, ip, last_used) VALUES ('YOUR UDID', '::ffff:10.55.0.5', CURRENT_TIMESTAMP);```
```INSERT INTO DEVICES (udid, ip, last_used) VALUES ('YOUR UDID', '::ffff:10.55.0.6', CURRENT_TIMESTAMP);```


Ex.
```INSERT INTO DEVICES (udid, ip, last_used) VALUES ('00008111-111122223333801E', '::ffff:10.55.0.2', CURRENT_TIMESTAMP);```

Now you can quickly check "Did I do this correctly?" (The casing is important here)

**Note: That semicolon on the end (;) is REQUIRED for this command to work correctly! We love our ; Don't drop it**

```SELECT * FROM devices;```

You should see something like this

```
::ffff:10.55.0.2|00008111-111122223333801E|2025-01-31 15:17:50
::ffff:10.55.0.3|00008111-111122223333801E|2025-01-31 15:17:50
::ffff:10.55.0.4|00008111-111122223333801E|2025-01-31 15:17:50
::ffff:10.55.0.5|00008111-111122223333801E|2025-01-31 15:17:50
::ffff:10.55.0.6|00008111-111122223333801E|2025-01-31 15:17:50
```

If you got the above response, then you are done with database creation. 

4. Press "Ctrl Key + D" (two keys) to exit from the sqlite screen.

## Shortcut

### Next
Follow the rest of the tutorial from [here](https://github.com/jkcoxson/JitStreamer-EB/blob/master/install-docs/jitstreamer-eb-debian-docker-instructions.md#part-ii---the-execution). We will set up the shortcut after this step.

### Setting Up the Shortcut on Your iDevice

1. On your iPhone or iPad, go to this [site](https://jkcoxson.com/jitstreamer)

2. Go to the bottom. Download that Shortcut. You will also need the Shortcuts app for iOS, obviously. Download it if you need to from the Apple App Store.

3. Open Shortcuts on iOS

4. Locate the JitStreamer EB shortcut in the list

5. Long press on it

6. Select the option "Edit"

7. Locate the IP section that needs changed. Under the long introduction from jkcoxson, you'll see a section with a yellow-colored icon called "Text". In the editable area it will have http://[an-ip]:9172 . This is the area we will be changing.

8. Change it so it is ```10.55.0.1```. Example:

```http://10.55.0.1:9172```

9. Hit "Done" in the upper-right corner.

# Running
Use a type c to c cable and connect the raspberry pi power port to the type c ipad. Go to setting and wait until a ethernet device pops up. Click the ethernet device and when you see an ip address that is ```10.55.0.x``` instead of something random, you are ready to run the shortcut. 

# F&Q 
### How does this work without wifi?
The type c cable acts like a ethernet cable between the devices, creating a local network. 


As always, contribute if you find something more.
