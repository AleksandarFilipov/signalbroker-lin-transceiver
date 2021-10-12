# LIN-transceiver for RaspberryPi 4

This application will communicate with a LIN-bus using with help from the Beamybroker. 

## Configuration
The application uses the lin_config.toml file to set up the rib-id and physical UART port
for the lin-transceiver on the RPI board. 4 ports can be configured in the file. 

Example Usage:
[[lin_ports]]         *Field that tells Rust that this goes into lin_ports vector*  
uart = "/dev/ttyAMA3" *This is the UART hardware port on the raspbery pi*
rib_id = 4            *this is the rib id used by Beamybroker*  

