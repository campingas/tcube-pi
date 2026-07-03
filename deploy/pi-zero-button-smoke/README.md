# Raspberry Pi Zero Button Smoke Payload

**Superseded:** the release bundle now installs the real GPIO runtime as `tcube-pi.service`, and the release installer disables `tcube-button-smoke.service` when it finds it. Use this payload only for low-level wiring diagnostics independent of the Rust runtime; the two services conflict over the GPIO lines and the ALSA device, so stop `tcube-pi.service` first.

This payload is an intermediate bench test for one physical button plus the MAX98357A I2S amplifier and speaker. It uses Linux `gpiomon` plus ALSA playback so bench hardware can prove that a physical button triggers local approved audio without involving the Rust runtime.

## Files Installed On The Pi

The installer copies files to these exact paths:

```text
/opt/tcube/content/
/opt/tcube/bin/tcube-button-smoke
/etc/tcube/tcube-button-smoke.env
/etc/systemd/system/tcube-button-smoke.service
/boot/firmware/config.txt
```

The boot config receives this I2S audio block if it is not already present:

```text
# T-Cube MAX98357A I2S audio
dtparam=i2s=on
dtoverlay=max98357a
```

## Build The Payload On The Mac

From the repo root:

```sh
rm -rf /tmp/tcube-pi-payload
mkdir -p /tmp/tcube-pi-payload
cp -R deploy /tmp/tcube-pi-payload/
cp -R content /tmp/tcube-pi-payload/
```

## Copy Over USB OTG Networking

If the Pi is reachable over USB gadget Ethernet:

```sh
scp -r /tmp/tcube-pi-payload pi@raspberrypi.local:/home/pi/tcube-pi-payload
ssh pi@raspberrypi.local
sudo /home/pi/tcube-pi-payload/deploy/pi-zero-button-smoke/install-on-pi.sh /home/pi/tcube-pi-payload
sudo reboot
```

If the hostname does not resolve, use the Pi USB network IP instead of `raspberrypi.local`.

## Copy Directly Onto A Mounted Micro SD Card

macOS normally mounts only the FAT boot partition, not the Linux ext4 root partition. To copy everything directly to the card, use a Linux machine or VM that mounts both partitions.

Copy these paths onto the mounted card:

```text
/tmp/tcube-pi-payload/content/ -> <rootfs>/opt/tcube/content/
/tmp/tcube-pi-payload/deploy/pi-zero-button-smoke/tcube-button-smoke -> <rootfs>/opt/tcube/bin/tcube-button-smoke
/tmp/tcube-pi-payload/deploy/pi-zero-button-smoke/tcube-button-smoke.env -> <rootfs>/etc/tcube/tcube-button-smoke.env
/tmp/tcube-pi-payload/deploy/pi-zero-button-smoke/tcube-button-smoke.service -> <rootfs>/etc/systemd/system/tcube-button-smoke.service
```

Then edit the boot partition file:

```text
<bootfs>/config.txt
```

On newer Raspberry Pi OS images this same file appears after boot as:

```text
/boot/firmware/config.txt
```

Add:

```text
# T-Cube MAX98357A I2S audio
dtparam=i2s=on
dtoverlay=max98357a
```

After first boot, enable the service:

```sh
sudo apt update
sudo apt install -y ca-certificates alsa-utils gpiod mpg123
sudo chmod +x /opt/tcube/bin/tcube-button-smoke
sudo systemctl daemon-reload
sudo systemctl enable --now tcube-button-smoke.service
```

## Configure The Button And Sound Folder

Edit:

```sh
sudo nano /etc/tcube/tcube-button-smoke.env
```

Default settings:

```sh
BUTTON_GPIO=17
AUDIO_DIR=/opt/tcube/content/audio/english
ALSA_DEVICE=plughw:CARD=MAX98357A,DEV=0
```

Use `BUTTON_GPIO=17` for the breadboard starter wiring and the five-button assembly button 1 (red): BCM GPIO17, physical pin 11.

Use `BUTTON_GPIO=5` for the five-button assembly button 4 (blue): BCM GPIO5, physical pin 29.

Use `BUTTON_GPIO=4` if your bench wiring follows the older breadboard smoke diagram label: BCM GPIO4.

Set `AUDIO_DIR` to one of:

```text
/opt/tcube/content/audio/english
/opt/tcube/content/audio/animals
/opt/tcube/content/audio/music
```

Keep `ALSA_DEVICE=plughw:CARD=MAX98357A,DEV=0` when `aplay -l` shows a `MAX98357A` card. This avoids the HDMI audio device becoming the default playback target.

Restart after changes:

```sh
sudo systemctl restart tcube-button-smoke.service
```

## Verify

Check the service:

```sh
systemctl status tcube-button-smoke.service
journalctl -u tcube-button-smoke.service -f
```

Check the audio card:

```sh
aplay -l
speaker-test -D plughw:CARD=MAX98357A,DEV=0 -t wav -c 2 -l 1
```

Play one content file manually:

```sh
aplay -D plughw:CARD=MAX98357A,DEV=0 /opt/tcube/content/audio/english/good-job.wav
mpg123 /opt/tcube/content/audio/music/race-car.mp3
```

Check the button line manually:

```sh
gpiomon -n 1 -r gpiochip0 17
```

Press the button once. If your wire is on another line, replace `17` with its BCM number.
