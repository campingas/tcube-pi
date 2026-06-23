# T-Cube V1 KiCad Project

Open `t-cube-v1.kicad_pro` in KiCad 10. The project contains a native connector-level schematic and a routed 65 mm × 56 mm, two-layer carrier PCB for the Raspberry Pi Zero 2 W, five MKE-M02 button modules, one MAX98357A amplifier module, and one 8-ohm speaker.

The Hshop products are assembled modules, so the carrier uses project-local connector footprints in `T_Cube.pretty` rather than attempting to reproduce their onboard circuits. Confirm the pin labels and physical pitch on the delivered modules before ordering or assembling the PCB.

## Connectors

| Reference | Device | Pin order |
| --- | --- | --- |
| `J1` | Raspberry Pi Zero 2 W J8 | Standard physical pins 1–40 |
| `J2`–`J6` | MKE-M02 buttons 1–5 | 1 `GND (-)`, 2 `+5V (+)`, 3 `S` |
| `J7` | MAX98357A module | 1 `VIN`, 2 `GND`, 3 `BCLK`, 4 `LRC`, 5 `DIN`, 6 `OUT+`, 7 `OUT-` |
| `J8` | 3 W, 8-ohm speaker | 1 `SPK+`, 2 `SPK-` |

The speaker output is differential. Neither speaker terminal may be connected to ground.

## GPIO Mapping

| Function | BCM GPIO | Pi physical pin |
| --- | ---: | ---: |
| Button 1 | GPIO5 | 29 |
| Button 2 | GPIO6 | 31 |
| Button 3 | GPIO17 | 11 |
| Button 4 | GPIO27 | 13 |
| Button 5 | GPIO16 | 36 |
| I2S BCLK | GPIO18 | 12 |
| I2S LRC | GPIO19 | 35 |
| I2S DIN | GPIO21 | 40 |

## Board Notes

- Board outline: 65 mm × 56 mm.
- Stackup: two copper layers, nominal 1.6 mm board thickness.
- Mounting: four 2.7 mm non-plated holes intended for M2.5 hardware.
- Power: bottom-layer `+5V` plane and top-layer ground plane.
- Signals: 0.35 mm traces; differential speaker traces are 0.50 mm.
- The Pi and module headers are through-hole footprints. Verify which board face each physical module must occupy before soldering sockets or pin headers.

## Validation

Validated with KiCad CLI 10.0.3 on 2026-06-20:

- ERC: 0 errors and 0 warnings.
- DRC: 0 violations and 0 unconnected items.
- Schematic/PCB parity: 0 issues.
- Board zones refilled and the project rendered successfully.

These checks validate the documented netlist and PCB rules, not the dimensions of the delivered Hshop breakout boards. Measure those boards and update the project-local footprints if their connector positions or mechanical clearances differ.
