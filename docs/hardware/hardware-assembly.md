# Hardware Assembly

This file tracks the physical components selected or seriously considered for T-Cube and the current prototype assembly path.

Maintenance rule: whenever a hardware piece, device, module, or major material is added, removed, or replaced, update this file in the same change.

## Hardware Inventory

| Name | Short description |
| --- | --- |
| Raspberry Pi Zero 2 W Basic Kit | Main computer for running the device software and controlling connected hardware. |
| MakerEdu MKE-M02 Button RGYBW Module | Development button for testing physical input through GPIO. |
| MAX98357A I2S Class-D 3 W amplifier | Amplifies digital audio from the Raspberry Pi for the speaker. |
| Mini 3 W 8-ohm speaker with enclosure | Plays speech, animal sounds, music, and device feedback. |
| MPU6050 GY-521 6-DOF IMU | Detects movement, rotation, impact, and cube orientation. |
| 830-point solderless breadboard | Holds temporary circuits during prototype testing. |
| 20 cm male-to-female jumper wires, 40-wire ribbon | Connects Raspberry Pi GPIO pins to prototype modules. |
| Micro-USB to USB-C OTG cable | Connects a USB-C peripheral to the Raspberry Pi Zero 2 W data port. |

## Assembly Instructions

New to electronics or the Raspberry Pi? Start with the one-button walkthrough in [Breadboard Starter Wiring](breadboard-starter-wiring.md), then come back here for the full five-button build.

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

Insert a microSD card flashed with Raspberry Pi OS Lite. Power the Pi via the PWR Micro-USB port using a 5V/2.5A supply. Complete initial OS setup for hostname, Wi-Fi, and SSH. Then follow [Raspberry Pi OS Lite Install](pi-os-lite-install.md) for package installation, release-bundle installation, I2S configuration, and service checks.

**Tips**
- Keep the Pi reachable over SSH before enclosing or mounting hardware.
- After reboot, run `aplay -l` — you should see a `sndrpimaxims` I2S sound card listed.

**Warnings**
- Use the correct Micro-USB port labelled **PWR** — the other port (USB OTG) is for data only.

---

### Step 3: Connect the MAX98357A amplifier to the Pi

Place the MAX98357A breakout on the breadboard. Wire it to the Pi's 40-pin header using jumper wires:

- **LRC** → Pi physical pin 35 (BCM19)
- **BCLK** → Pi physical pin 12 (BCM18)
- **DIN** → Pi physical pin 40 (BCM21)
- **GAIN** → empty
- **SD** → Pi physical pin 7 (BCM4)
- **GND** → Pi physical pin 6 (GND)
- **VIN** → Pi physical pin 2 (5V)

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

### Step 6: Validate hardware bring-up

**Tips**
- Install the Pi software path from [Raspberry Pi OS Lite Install](pi-os-lite-install.md) before validating end-to-end admin service behavior.
- Use `aplay -l` and `speaker-test` or a short known-good WAV file to validate the MAX98357A and speaker path before relying on the Rust runtime.
- Use the temporary one-button payload under `deploy/pi-zero-button-smoke` only while the final Rust GPIO backend is pending.

**Warnings**
- If no audio plays, check `aplay -l` to confirm the I2S sound card is visible. If not, verify the `dtoverlay` lines in `/boot/config.txt` and reboot.
- Power down before changing amplifier, speaker, or GPIO wiring.

## Recommended Bring-Up Order

1. With power disconnected, verify continuity and confirm there is no short between 5 V and ground.
2. Connect only Button 1, boot the Pi, and verify low when idle and high when pressed.
3. Add Buttons 2 through 5 one at a time and verify each assigned GPIO independently.
4. Power down, connect amplifier power and I2S signals without the speaker, then inspect the wiring again.
5. Connect the speaker between `OUT+` and `OUT-`, start at low software volume, and run a short playback test.
6. Test simultaneous button input and audio playback while watching for undervoltage, resets, noise, or false button events.

---
