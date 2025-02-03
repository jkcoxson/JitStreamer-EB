# How to Self-Host JITStreamer-EB with Tailscale 
Thank you to [jkcoxson](https://github.com/jkcoxson) for creating this project, [Unlearned6688](https://github.com/Unlearned6688) for creating [jitstreamer-eb-debian-docker-instructions.md](https://github.com/jkcoxson/JitStreamer-EB/blob/master/jitstreamer-eb-debian-docker-instructions.md) which I will be referencing multiple times through this instructions, and others who had contributed to this project.

You will first need to follow [these instructions](https://github.com/jkcoxson/JitStreamer-EB/blob/master/jitstreamer-eb-debian-docker-instructions.md#prerequisites) until you reach [this section](https://github.com/jkcoxson/JitStreamer-EB/blob/master/jitstreamer-eb-debian-docker-instructions.md#create-your-database-file) (meaning don't do the steps in the second link).

### Installing Tailscale
First install tailscale **AND ACTIVATE IT** on your iDevice. Watch [this](https://www.youtube.com/watch?v=sPdvyR7bLqI) if you need help. 

On your host device, run the following to install tailscale and authenticate to connect it to your account.
Linux
```
curl -fsSL https://tailscale.com/install.sh | sh
tailscale up
```
If you are using any other devices go to [this website](https://tailscale.com/download) and continue the instructions from there

Write down the host device's Tailscale IP and iDevice's Tailscale IP. DO NOT MIX THEM UP

### Create Your Database File
For this step, I will be heavily referencing and slightly modifying [this section](https://github.com/jkcoxson/JitStreamer-EB/blob/master/jitstreamer-eb-debian-docker-instructions.md#create-your-database-file).

This part might be a bit weird. Prepare yourself.

1. You have to create the jitstreamer.db file (sqlite database) using build instructions included in the repo.

```
mkdir app
sqlite3 ./jitstreamer.db < ./src/sql/up.sql
```

2. Now you have a fancy little database. But you need to add your device info into it.

Note: There are likely many ways to achieve the desired result here. This is just how I did it. It may be more steps than required, but it lets you see how the database works internally which I prefer for myself.

Type into the terminal

``sqlite3``

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

``.open jitstreamer.db``

Optional: "Is everything ok so far?" check. Type:

``.tables``

You should see:

``devices       downloads     launch_queue  mount_queue``

This means your tables inside the database were created as instructed above. Good, good.

3. **Read the entire next part, including the notes and everything. Don't just blindly copy/paste! You must edit stuff!**

Type this command from the repository to add your device information to the DEVICES table.

``INSERT INTO DEVICES (udid, ip, last_used) VALUES ([udid], [ip], CURRENT_TIMESTAMP);``

Replace the [udid] and [ip] (so, the second set. The two with the brackets!) with (examples)'00008111-111122223333801E' and '100.100.35.52'

Note 1: The above UDID is FAKE. INSERT YOUR OWN UDID! I used a fake one which resembles a real one to help visually. Please... don't copy that into your database.

Note 2: The brackets are now deleted. They are replaced with ' (NOT ")

Note 3: The IP in question is the **TAILSCALE** IP of the iDevice. The **TAILSCALE** IP of your iPhone, etc. Each device needs to have a different IP.

Note 4: You **HAVE** to add "::ffff:" in front of the  **TAILSCALE** IP address. eg: ``::ffff:100.100.35.52``

This is a FAKE but realistic example. Yours will contain your own UDID and IP.

``INSERT INTO DEVICES (udid, ip, last_used) VALUES ('00008111-111122223333801E', '::ffff:100.100.35.52', CURRENT_TIMESTAMP);``

Now you can quickly check "Did I do this correctly?" (The casing is important here)

**Note: That semicolon on the end (;) is REQUIRED for this command to work correctly! We love our ; Don't drop it**

``SELECT * FROM devices;``

You should see something like this for each device you inserted above.

``::ffff:100.100.35.52|00008111-111122223333801E|2025-01-31 15:17:50``

If you got the above response, then you are done with database creation.

4. Press "Ctrl Key + D" (two keys) to exit from the sqlite screen.


### Next Steps
Continue following the steps linked [here](https://github.com/jkcoxson/JitStreamer-EB/blob/master/jitstreamer-eb-debian-docker-instructions.md#part-ii---the-execution) until [here](https://github.com/jkcoxson/JitStreamer-EB/blob/master/jitstreamer-eb-debian-docker-instructions.md#setting-up-the-shortcut-on-your-idevice).

### Setting Up Your iDevice

* Go to this [site](https://jkcoxson.com/jitstreamer)
* Go to the bottom. Download that Shortcut. You will also need the Shortcuts app for iOS, obviously.
* Open Shortcuts on iOS
* Locate the JitStreamer EB shortcut in the list
* Long press on it
* Select the option "Edit"
* Locate the IP section that needs changed. Under the long introduction from jkcoxson, you'll see a section with a yellow-colored icon called "Text". In the editable area it will have http://[an-ip]:9172
* Change it so that it matches the TAILSCALE IP of your HOST machine. The machine you are running Docker on. Example:
  ``http://100.168.10.37:9172``
  Obviously the IP would be your own IP.
* Hit "Done" in the upper-right corner.

Continue on from [here](https://github.com/jkcoxson/JitStreamer-EB/blob/master/jitstreamer-eb-debian-docker-instructions.md#oh-yeah-its-all-coming-together-time-to-jit)

This is not the best guide ever written but feel free to contribute and edit this.