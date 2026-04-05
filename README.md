# offline-bitcoin-atm
Bitcoin ATM (coins only) with lightning network support, running offline on an esp32. 
The lightning network is a 2nd layer protocol on top of the bitcoin protocol enabling trustless transactions with instant settlement and cheap fees.

## Special thanks to
@21isenough and contributors for the 3d models and inspiration  
[RPi based Lightning ATM repo](https://github.com/21isenough/LightningATM)

LNBits, Ben Arc and Stepan Snigirev for the cryptograpy used to make this ATM working without internet connection  
[Fossa ATM repository](https://github.com/lnbits/fossa) | [LNBits Homepage](https://lnbits.com/)

Axel Hamburch for the very detailed guide in german language on assembly of the electronics  
[Ereignishorizont Blog](https://ereignishorizont.xyz/lightning-atm/)

@marcelhino for the OrangeClock project, whose code was used as reference for the "BlockClock" idle display mode.
[OrangeClock repository](https://github.com/marcelhino/orangeclock)

## Web Flasher

Flash the firmware directly from your browser without installing any tools.
Open the **[Web Flasher](https://f321x.github.io/offline-bitcoin-atm/)** in Chrome or Edge, connect your ESP32 via USB, and click "Install Firmware".

After flashing, the ATM starts a WiFi access point (`LightningATM`) for configuration — see step 4 below.

## Used parts
All the parts are available on eBay and Aliexpress

* ESP32 NodeMCU Dev Board | Any "normal" esp32 dev board should do the job here | [Example](https://web.archive.org/web/20231202141343/https://www.berrybase.de/en/esp32-nodemcu-development-board)
* DC-DC Adjustable Step-up Boost Power Supply LM2587S 5V -> 12V | for the coin acceptor, runs on 12V [Example](https://de.aliexpress.com/item/32834930982.html)
* Waveshare 1.54 inch e-Paper Display Modul with SPI Interface | [Example](https://www.waveshare.com/1.54inch-e-paper-module.htm)
* Programmable Coin Acceptor (HX-616) - 6 Coin | [Example](https://de.aliexpress.com/item/1005005203759184.html)
* 10mm Metal Push Button Switch 3-6V with Yellow LED, Self-reset Momentary | [Example](https://de.aliexpress.com/item/1005004527235094.html)
* USB Type C socket | to plug in the power supply (i used a Raspberry Pi type C power supply) [Example](https://de.aliexpress.com/item/1005005347655323.html)
* Little Mosfet modules ("15A 400W MOS FET Trigger") | To block the coin acceptor at certain points [Example](https://de.aliexpress.com/item/33038160184.html)
* Orange PLA Filament for the 3D Printer | [Example](https://us.polymaker.com/products/polylite-pla)
* Jumper Wires | [Example](https://de.aliexpress.com/item/1005005945668553.html)
* Heat-Set Threaded Inserts M3 | [Example](https://www.prusa3d.com/product/threaded-inserts-m3-standard-100-pcs/)

All in all would calculate around $100 for the neccessary parts

# Assembly Instructions for Lightning ATM

## 1. Connecting the Waveshare 1.54 inch Display to the ESP32

Begin by connecting the Waveshare display to the ESP32 using the following pin assignments:

| Display Pins | ESP32 GPIO |
|--------------|------------|
| Busy         | 27         |
| RST          | 33         |
| DC           | 25         |
| CS           | 26         |
| CLK          | SCK = 18   |
| DIN          | MOSI = 23  |
| GND          | GND        |
| 3.3V         | 3.3V       |

## 2. Programming the Coin Acceptor

Ensure to adjust the voltage of the step-up converter before connecting the coin acceptor. Detailed programming instructions are available in the following guides:
- [Coin Acceptor Programming Guide (English)](https://github.com/21isenough/LightningATM/blob/master/docs/guide/coin_validator.md)
- [Coin Acceptor Programming Guide (German)](https://ereignishorizont.xyz/lightning-atm/)

## 3. Connecting the Coin Acceptor to the ESP32

- For the Coin <-> Pin 17 connection, use a cable as short as possible.
- Short circuit the two pins below the switch on the coin acceptor with the MOSFET on GND IN and GND OUT.
- Connect the MOSFET GND pin to the ESP32 GND and the PWM pin to the pin specified in `src/board.rs`.

| Periphery Pin            | ESP32 GPIO |
|--------------------------|------------|
| Coin Acceptor 'Coin Pin' | 17         |
| Mosfet PWM Pin           | 16         |
| Mosfet GND Pin           | GND        |


## 4. Connecting the LED Button

Connect the LED Button to the ESP32 according to the pin assignments specified in `src/board.rs`:

| Periphery Pin             | ESP32 GPIO |
|---------------------------|------------|
| Button LED Pin (+)        | 21         |
| Button LED Pin (-)        | GND        |
| Button PIN 1              | 32         |
| Button PIN 2              | GND        |

For wiring inspiration and guidance, refer to [Lightning ATM Documentation](https://github.com/21isenough/LightningATM/tree/master/docs).

## 5. Circuit Diagram

Below are two possible wiring options. Depending on which ESP32 type and display / driver board you are using. 

1. Standard ESP32 with separate display / driver board

![Wiring - Circuit Diagram Normal ESP32](./assets/schematics/offlineATM-xxxxxx.png)

2. Waveshare ESP32 with integrated driver board and separate display

![Wiring - Circuit Diagram Waveshare ESP32](./assets/schematics/offlineATM-WaveshareESP32-xxxxxx.png)

## 6. Setup software

1. Install the Rust ESP32 toolchain (if you don't have `cargo` yet, install Rust via [rustup](https://rustup.rs/)):
    ```bash
    cargo install espup --locked
    espup install
    cat $HOME/export-esp.sh >> ~/.bashrc
    source ~/.bashrc
    cargo install espflash ldproxy
    ```

2. Clone this repository and build:
    ```bash
    git clone https://github.com/f321x/offline-bitcoin-atm.git
    cd offline-bitcoin-atm
    cargo build --release
    ```

3. Create an [LNbits](https://lnbits.com/) wallet. Add the **FOSSA** extension and create a new ATM connection in the Extension by clicking on **NEW FOSSA**.

    [![wallet_settings_02_thumb](./assets/wallet-config/wallet_settings_01_thumb.png)](./assets/wallet-config/wallet_settings_01.png)

4. Copy the FOSSA connection string. The ATM stores configuration in flash memory - on first boot it will start a WiFi access point for configuration where you can enter the connection string. Connect to the `LightningATM` WiFi network and open [http://atm.local](http://atm.local) in your browser.

    > **Tip:** To reconfigure the ATM later, hold the **BOOT button** (GPIO0) on the ESP32 during power-on. The device will re-enter the WiFi configuration portal. The BOOT button is located on the ESP32 board inside the enclosure and is not accessible to end users.

    [![wallet_settings_02_thumb](./assets/wallet-config/wallet_settings_02_thumb.png)](./assets/wallet-config/wallet_settings_02.png)

5. Flash the software on the esp32. You may have to disconnect the ESP32 from the step up converter before connecting it to the computer to prevent faults, or power it up with the power supply and use an usb isolator.
    ```bash
    cargo run --release  # flashes and opens serial monitor
    ```

If you need help ask me on Nostr @npub1z9n5ktfjrlpyywds9t7ljekr9cm9jjnzs27h702te5fy8p2c4dgs5zvycf

If this software and guide provided value to you feel free to send some sats to x@lnaddress.com


## Images

Standard ESP32
![PXL_20231126_144603980](https://github.com/f321x/offline-LightningATM-esp32/assets/51097237/12ac8a54-8756-4842-b26d-4408e8df3afe)
![PXL_20231126_162906807 MP_1](https://github.com/f321x/offline-LightningATM-esp32/assets/51097237/7e394774-f341-4b1c-ae73-4806f6f42ce5)

Waveshare ESP32
![Construction - Waveshare ESP32](./assets/photos/WaveshareESP32_construction_005.jpg)
![Construction - Waveshare ESP32](./assets/photos/WaveshareESP32_construction_001.jpg)
![Construction - Waveshare ESP32](./assets/photos/WaveshareESP32_construction_002.jpg)
![Construction - Waveshare ESP32](./assets/photos/WaveshareESP32_construction_003.jpg)
![Construction - Waveshare ESP32](./assets/photos/WaveshareESP32_construction_004.jpg)
