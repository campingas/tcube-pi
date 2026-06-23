# Hardware Inventory

This file tracks the physical components selected or seriously considered for T-Cube.

Maintenance rule: whenever a hardware piece, device, module, or major material is added, removed, or replaced, update this file in the same change.

## First Bench Prototype

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

## Recommended Bring-Up Order

1. With power disconnected, verify continuity and confirm there is no short between 5 V and ground.
2. Connect only Button 1, boot the Pi, and verify low when idle and high when pressed.
3. Add Buttons 2 through 5 one at a time and verify each assigned GPIO independently.
4. Power down, connect amplifier power and I²S signals without the speaker, then inspect the wiring again.
5. Connect the speaker between `OUT+` and `OUT-`, start at low software volume, and run a short playback test.
6. Test simultaneous button input and audio playback while watching for undervoltage, resets, noise, or false button events.
