# Assembly instructions

---

### Step 1: Solder the 40-pin GPIO header to the Pi Zero 2W

If your Pi Zero 2W does not have a pre-soldered header, solder a 2×20 male 40-pin header to the board. Use a breadboard to hold the header straight while soldering. Pin 1 is marked by a square pad and a small arrow on the silkscreen.

**Tips**
- Work with the Pi face-up; pins point down through the board into the breadboard below.
- Use a fine-tip soldering iron at ~350°C. Make clean cone-shaped joints — cold joints cause intermittent faults.

**Warnings**
- Do NOT power the Pi at any point during soldering.
- Identify Pin 1 carefully — reversing polarity can destroy the board.

---

### Step 2: Power the Pi Zero 2W and prepare the OS

Insert a microSD card flashed with Raspberry Pi OS Lite. Power the Pi via the PWR Micro-USB port using a 5V/2.5A supply. Complete initial OS setup (hostname, Wi-Fi, SSH). Then open a terminal and enable I2S audio by editing `/boot/config.txt`: add the lines `dtparam=i2s=on` and `dtoverlay=max98357a`, then reboot.

**Tips**
- Use `sudo nano /boot/config.txt` to edit. Save with **Ctrl+O**, exit with **Ctrl+X**.
- After reboot, run `aplay -l` — you should see a `sndrpimaxims` I2S sound card listed.

**Warnings**
- Use the correct Micro-USB port labelled **PWR** — the other port (USB OTG) is for data only.

---

### Step 3: Connect the MAX98357A amplifier to the Pi

Place the MAX98357A breakout on the breadboard. Wire it to the Pi's 40-pin header using jumper wires:

- **VIN** → Pi physical pin 2 (5V)
- **GND** → Pi physical pin 6 (GND)
- **BCLK** → Pi physical pin 12 (BCM18)
- **LRC** → Pi physical pin 35 (BCM19)
- **DIN** → Pi physical pin 40 (BCM21)
- **SD** → Pi physical pin 7 (BCM4)

**Components:**
- MAX98357A I2S Class-D Mono Amplifier Breakout

**Tips**
- Use the colour-coded BCM pinout diagram at [pinout.xyz](https://pinout.xyz) to find physical pin numbers.
- Add a 100µF electrolytic capacitor across VIN and GND on the breadboard near the MAX98357A to smooth the 5V supply and reduce audio hiss.

**Warnings**
- Double-check VIN polarity — reversing the power supply will destroy the MAX98357A chip.
- Do not connect the speaker yet — finish all wiring first.

---

### Step 4: Connect the speaker to the MAX98357A

Connect the mini 3W 8-ohm speaker wires to the **SPK+** and **SPK−** terminals on the MAX98357A breakout. The terminal block uses a small flathead screwdriver to clamp the wires.

**Components:**
- MAX98357A I2S Class-D Mono Amplifier Breakout
- Mini 3W 8-Ohm Speaker with Enclosure

**Tips**
- Strip 5–6 mm of insulation from each speaker wire before inserting.
- Polarity matters for stereo phase but not for mono playback — either orientation works.

**Warnings**
- Never connect or disconnect the speaker while the amplifier is powered and playing audio — this can cause a loud pop or damage the output stage.

---

### Step 5: Connect the 5 MKE-M02 push-button modules

Each MKE-M02 module has a 3-pin XH2.54 connector: VCC, GND, SIG. Wire all five buttons as follows:

- **Red button (btn1):**  
  VCC → Pi 3.3V (pin 1 or 17), GND → Pi GND, SIG → Pi physical pin 11 (BCM17)

- **Green button (btn2):**  
  VCC → 3.3V, GND → GND, SIG → Pi physical pin 13 (BCM27)

- **Yellow button (btn3):**  
  VCC → 3.3V, GND → GND, SIG → Pi physical pin 15 (BCM22)

- **Blue button (btn4):**  
  VCC → 3.3V, GND → GND, SIG → Pi physical pin 29 (BCM5)

- **White button (btn5):**  
  VCC → 3.3V, GND → GND, SIG → Pi physical pin 31 (BCM6)

**Components:**
- MakerEdu MKE-M02 Push Button Module (×5)

**Tips**
- Use a small breadboard power rail for the 3.3V and GND shared connections.
- The MKE-M02 has an onboard pull-down resistor — no external resistor needed.

**Warnings**
- Connect buttons to the **3.3V** rail (NOT 5V) — the Pi GPIO pins are 3.3V max; applying 5V will destroy them.
- Keep SIG wires short to reduce noise pickup.

---

### Step 6: Add sound files and run the script

Copy five WAV audio files to the same folder as `main.py` on the Pi and rename them:  
`sound_red.wav`, `sound_green.wav`, `sound_yellow.wav`, `sound_blue.wav`, `sound_white.wav`.

Then install the required Python libraries and run the script:

```bash
sudo apt update && sudo apt install -y python3-gpiozero python3-pygame
python3 main.py
```

Press each button — the corresponding sound should play through the speaker.

**Tips**
- 44100 Hz, 16-bit mono WAV files work best. You can convert any audio file with:
  ```bash
  ffmpeg -i input.mp3 -ar 44100 -ac 1 -sample_fmt s16 sound_red.wav
  ```
- To make the script run automatically at startup, add it to `/etc/rc.local` before the `exit 0` line.

**Warnings**
- If no audio plays, check `aplay -l` to confirm the I2S sound card is visible. If not, verify the `dtoverlay` lines in `/boot/config.txt` and reboot.

---

