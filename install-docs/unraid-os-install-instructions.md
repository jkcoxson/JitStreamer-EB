# How to Self-Host JITStreamer-EB (For Dummies) (Now in unRAID Flavor!)

## What is this?

I write these little guides for myself and sometimes like to share. My goal is only to provide more precise and full instructions to those who have very little to no knowledge of the required tools.

This guide will be going over how to get JITStreamer-EB (I'll refer to it as the "project" in the future to shorten my typing) up and running in Docker. Specifically using Docker Compose. Docker Compose offers, in my opinion, a much easier deployment of Docker containers. This is absolutely not the only way to run this project.

**Note: This is a condensed version for unRAID OS. I did tbis quickly and made it by cutting out a ton of stuff not neede to run
it on unRAID OS.**

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

## Part I - Preparation

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

**NOTE: I don't recommend doing any of this below in unRAID. unRAID isn't really meant to be used for these types of operations. You probably can (I haven't tested), but just to be clear: I don't think you should. You can make the .plist files in a VM, on a regular Windows boot, etc. and transfer them to your unRAID install.

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

1. You have to create the jitstreamer.db file (sqlite database) using build instructions included in the repo. I modified this a bit from the Debian instructions I had written previously since you aren't cloning the repo on unRAID. There's likely a faster/better way to do this. This will work though.

Make the database directory

```mkdir /mnt/user/appdata/jitstreamer-eb/app```

Make the database file

```touch /mnt/user/appdata/jitstreamer-eb/app/jitstreamer.db```

2. Now you have a fancy little database. But you need to add your device info into it.
Note: There are likely many ways to achieve the desired result here. This is just how I did it. It may be more steps than required, but it lets you see how the database works internally which I prefer for myself.
Type into the terminal
```sqlite3 /mnt/user/appdata/jitstreamer-eb/app/jitstreamer.db```
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

**NOTE: This part is different for unRAID. Make sure to copy the below code directly into the new database file!**

[This code](https://github.com/jkcoxson/JitStreamer-EB/blob/master/src/sql/up.sql) needs to be pasted into the now-open for editing database. Check the link for the most up-to-date code to ccpy/paste.

```
create table devices (
  ip varchar(15) primary key,
  udid varchar(40) not null,
  last_used datetime not null
);


create table downloads (
  code varchar(40) primary key,
  contents varchar(255) not null
);

create table launch_queue (
  udid varchar(40) not null,
  ip varchar(32) not null,
  bundle_id varchar(255) not null,
  status int not null, -- 0: pending, 2: error
  error varchar(255),
  ordinal integer primary key
);
```

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

Note 4: You **HAVE** to add "::ffff:" in front of the regular IPv4 IP address. eg: 

```::ffff:192.168.1.2```

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
