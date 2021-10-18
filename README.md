# LIN-transceiver for RaspberryPi 4

This application will communicate with a LIN-bus using with help from the Beamybroker. 

## How to compile

First of all, wheter you want to cross-compile from your machine or if you want to compile directly to raspberry, you need to install rust, follow this steps: https://www.rust-lang.org/tools/install

If you are going to compile it using the raspberry pi you just need to this:
```zsh
# Unoptimized rust binary
$ cargo build
# Optimized and no debug symbols included
$ cargo build --release  
```

In order to cross-compile from your machine you follow theese steps:

```zsh
# Add armv7 target to rustup
rustup target add armv7-unknown-linux-gnueabihf

# Install armv7 toolchain for gcc
sudo apt install gcc-arm-linux-gnueabihf

# Then you can use the shellscript for both build and deploy on the raspberry pi by passing the ip
./deploy 192.168.1.101
```

## Configuration
The application uses the lin_config.toml file to set up the rib-id and physical UART port
for the lin-transceiver on the RPI board.
Up 4 ports can be configured in the file. 

### Example Usage

```toml
# Field that tells Rust that this goes into lin_ports vector
[[lin_ports]] 

# This is the UART hardware port on the raspbery pi
uart = "/dev/ttyAMA3"

# This is the rib id used by Beamybroker
rib_id = 4 
```