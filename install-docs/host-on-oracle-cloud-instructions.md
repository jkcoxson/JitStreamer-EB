# How to Self-Host JITStreamer-EB Using Oracle Cloud VM

## What... is this?

For a long time I have been pondering how to enable JIT without PC access. After numerous frustrations, I have concluded that there is no way to start from scratch without any PC usage. However, if you want to self-host JITStreamer-EB without PC after the initial setup, this is a not-so-detailed guide to offer you some help.

Another way to self-host without PC is using UTM SE as a server, and set up VPN tunnel between UTM SE and your iDevice. Unfortunately, UTM SE is painfully slow and may take you several hours (or days) to set up the environment. Additionally, UTM SE must be in the foreground to keep itself running, taking up a huge proportion of your monitor space. Therefore, UTM SE is somehow impratical for daily use, and thus this guide was born.

Some paragraphs were copied from the referenced guides. Big thanks to everyone contributed to this project.

### TL;DR

Pros

- You don't need a PC after generating a certain file.

- You don't need to run UTM SE. Faster and (far) less resource-intensive.

- Self-hosting. No queueing or server down time (unless Oracle bans you for random reasons).

Cons

- Oracle obtains your personal information during registration. And the cloud VM is under their control.

## Credit and Sources

The GOAT jkcoxson created this beautiful project for us. Here's his [GitHub](https://github.com/jkcoxson) and here's the specific [project](https://github.com/jkcoxson/JitStreamer-EB) of interest. Also its [website](https://jkcoxson.com/jitstreamer). He provided this to everyone for free. You should consider donating to him to show appreciation and support this type of work in the future.

Please consider checking the following guides when you run into errors!
This guide is heavily derived from them. Big thanks.

- [JITStreamer-EB Docker Installation Guide by Unlearned6688](https://github.com/jkcoxson/JitStreamer-EB/blob/master/install-docs/jitstreamer-eb-debian-docker-instructions.md)

- [JITStreamer-EB Tailscale Setup Guide by EnderRobber101](https://github.com/jkcoxson/JitStreamer-EB/blob/master/install-docs/jitstreamer-eb-debian-docker-tailscale-instructions.md)

[Oracle Cloud](https://www.oracle.com/cloud/) but of course you can choose other cloud VM providers. I chose Oracle because they provide free VMs with sufficient power to accomplish the task.

## The guide starts here!

### Oracle Cloud: From Signing Up to Creating Instance

1. Sign up [Oracle Cloud Free Tier](https://www.oracle.com/cloud/free/).

    - You have to fill in your credit card information in order to use the free service, but you won't be actually charged if you do not upgrade your tier (I think so). There is a test charge after you fill in your card info, it will not actually cost anything.

    - Of course you can choose other cloud computing provider as you wish.

2. After registration, create a VM instance on your dashboard with following arguments:

    - `Image`: I chose `Ubuntu 22.04 minimal`. But I think any distro should work as expected.

    - `Shape`: `VM.Standard.E2.1.Micro`. To ensure always-free usage.

    - `SSH keys`: `Generate a key pair for me` and `Save private key` because you will need to send a certain file via [SSH](https://wiki.archlinux.org/title/Secure_Shell).

3. Press the `Create` button.

4. You will be redirected to the `Instance details` page. After a short while your VM will turn green which means it's ready. 

5. Connect the VM by [SSH](https://wiki.archlinux.org/title/Secure_Shell) its public IP or via the web UI console (On the page showing `Instance details`, click `Resources` -> `Console connection` and then click `Launch Cloud Shell Connection`).

How to connect via SSH is not the main topic of this guide. You can take a look at the following references, depending on your operating system:

- Windows: [PuTTY](https://www.putty.org/); [Simple Guide](https://docs.cfengine.com/docs/3.25/getting-started-installation-pre-installation-checklist-putty-quick-start-guide.html)

- Mac (I do not own Mac so I cannot make sure whether it works or not): [Use built-in terminal](https://www.servermania.com/kb/articles/ssh-mac)

- [Linux](https://wiki.archlinux.org/title/Secure_Shell)

- iOS/iPadOS: I use [ShellFish](https://apps.apple.com/us/app/ssh-files-secure-shellfish/id1336634154).
    
Use the private key generated during instance creation to authenticate your SSH session.

If are using a command line interface, you could SSH using the following command.
Replace `path_to_your_private_key_file`, `vm_username` and `cloud_vm_public_ip` with the real ones you get.

```
ssh -i path_to_your_private_key_file vm_username@cloud_vm_public_ip
```

For example:

```
ssh -i ~/ssh-key-2025-02-29.key ubuntu@69.42.0.114
```

### Prerequisite 1: iDevice Paring File (This is the only step that requires a PC!)

#### [Jitterbugpair](https://github.com/osy/Jitterbug)

Follow the instructions from this page: [Sidestore](https://docs.sidestore.io/docs/getting-started/pairing-file)

Btw, arch users may use the [jitterbugpair-bin AUR](https://docs.sidestore.io/docs/getting-started/pairing-file).

After this step you will obtain a file named `YOURDEVICEUDID.mobiledevicepairing` (for example `00008111-111122223333801E.mobiledevicepairing`).

#### Rename the generated mobiledevicepairing file

Rename the extension (`mobiledevicepairing`) to `.plist`.

Now the filename should look like this: `00008111-111122223333801E.plist`

#### Copy to Cloud VM via SSH

Suppose you followed the guide and chose ubuntu for the operation system of your VM.
Then you could type the following command into your terminal and send the pairing file to the home folder of your VM.

```
scp -i path_to_your_private_key_file path_to_the_pairing_file ubuntu@cloud_vm_public_ip:/home/ubuntu/
```

For example:

```
scp -i ~/ssh-key-2025-02-29.key ~/00008111-111122223333801E.plist ubuntu@69.42.0.114:/home/ubuntu/
```

### Prerequisite 2: Docker

On your cloud VM (via SSH or cloud console),
follow [the official documentation](https://docs.docker.com/engine/install/debian/#install-using-the-repository) to setup Docker. 
Specifically, follow step 1. to 3. on the linked section.

### Prereuisite 3: Tailscale

We need Tailscale to conveniently connect iDevice to the cloud VM.

Follow instructions on [Tailscale website](https://tailscale.com/kb/1020/install-ios) to setup tailscale on your iDevice.

For the cloud VM, follow the [official guide](https://tailscale.com/kb/1031/install-linux).

### Prerequisite 4: JitStreamer EB Docker Image

This section is under construction.

1. clone the repository

```
git clone https://github.com/jkcoxson/JitStreamer-EB
```

2. change your working directory to the cloned repo.

```
cd JitStreamer-EB
```

3. provide the iDevice pairing file to JitStreamer-EB, remember to replace the example filename.

```
mkdir lockdown
mv ~/00008111-111122223333801E.plist lockdown
```

4. create database file

if the cloud vm does not have `sqlite3` installed, install it first: `sudo apt install sqlite3`

```
mkdir app
sqlite3 ./jitstreamer.db < ./src/sql/up.sql
sqlite3
```

When the terminal shows `sqlite>`,
type in the following command and execute them separately
(do not type multiple lines and execute at once)

```
.open jitstreamer.db
```
```
INSERT INTO DEVICES (udid, ip, last_used) VALUES ([udid], [ip], CURRENT_TIMESTAMP);
```

(Follwing notes are taken from [this guide](https://github.com/jkcoxson/JitStreamer-EB/blob/master/install-docs/jitstreamer-eb-debian-docker-instructions.md),
for troubleshooting and verifying the results,
please consult the guide.)

Replace the [udid] and [ip] (so, the second set. The two with the brackets!) with (examples)'00008111-111122223333801E' and '192.168.1.2'

Note 1: The above UDID is FAKE. INSERT YOUR OWN UDID! I used a fake one which resembles a real one to help visually. Please... don't copy that into your database.

Note 2: The brackets are now deleted. They are replaced with ' (NOT ")

Note 3: The IP in question is the TAILSCALE IP of the iDevice. The IP of your iPhone, etc. Each device needs to have a different IP.

Note 4: You HAVE to add "::ffff:" in front of the regular IPv4 IP address. eg: ::ffff:192.168.1.2

This is a FAKE but realistic example. Yours will contain your own UDID and IP:

`INSERT INTO DEVICES (udid, ip, last_used) VALUES ('00008111-111122223333801E', '::ffff:192.168.1.2', CURRENT_TIMESTAMP);`

`Ctrl D` to exit sqlite.

5. Build and run

`docker build -t jitstreamer-eb .`

`docker compose up -d`

### Prerequisite 5: The magical shortcut on your iDevice

1. download shortcut

On your iDevice (iPhone or iPad), visit the [JITStreamer-EB official site](https://jkcoxson.com/jitstreamer).

You don't need to `Download Wireguard`.
`Download Shortcut` alone is enough.

2. edit the shortcut

In the Shortcuts app, long press to edit the shortcut.

Delete all the steps that contains `Wireguard`.

(The following section is taken from [this guide](https://github.com/jkcoxson/JitStreamer-EB/blob/master/install-docs/jitstreamer-eb-debian-docker-tailscale-instructions.md).)

Under the long introduction from jkcoxson, you'll see a section with a yellow-colored icon called "Text". In the editable area it will have `http://[an-ip]:9172`

Change it so that it matches the TAILSCALE IP of your HOST machine. The machine you are running Docker on. Example: `http://100.168.10.37:9172` Obviously the IP would be your own IP.

Hit "Done" in the upper-right corner.

### Execution!!!!!

Execute the shortcut and then profit.

## You are strong.

Feel free to edit this guide and make it more detailed and comprehensive!