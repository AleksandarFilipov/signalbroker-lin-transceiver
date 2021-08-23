# LIN

How to setup the LIN transceiver

## Installation 

### Prerequisites


Download USB-drivers for the ESP-device, choose driver depending on OS here:
https://www.olimex.com/Products/IoT/ESP32/ESP32-POE/open-source-hardware 

For be available to upload this project to your device you need at least PlatformIO. 
You can either install it as a [CLI application](https://docs.platformio.org/en/latest/core/index.html#) but the prefferd way is to use their extension within an [IDE](https://docs.platformio.org/en/latest/integration/ide/pioide.html#).

I guess that the most popular is to use VS Code so I will use it in the example below.


#### VS Code

* [VS Code](https://code.visualstudio.com/)
* [PlatformIO Extension](https://marketplace.visualstudio.com/items?itemName=platformio.platformio-ide)

If you have installed the applications above, open this folder with VS Code.  

## Configuration

The thing you need to check/modify before upload is the rib id you assigning to the device. In order to do this, go to the main.cpp file placed in the src folder and change the variable. Remember the rib ID beacuse you need to setup the signalbroker server with this rib ID later. 

```cpp 
constexpr uint8_t rib_id = 1;
```

Now you are ready to upload this to your device!!

### Modify the interface.json on the server side

When you have uploaded the firmware to your ESP32-devices and assinged them with unique rib ID's. You need to configure the interfaces.json file on signalbroker-server side in the following way:

* namespace - Unique namespace name (Tip is to use Linbus and mode (master/slave) eg: LIN14-M)
* device_identifier - rib ID that you assigned to the device
* server/targetport - needs to be unique port for each device
* device_name - need as well be unique, to follow the example above for namespace: lin14m
* node_mode: master/slave depending on how the device is connected to the LIN bus
* ldf_file - paste the link to where you have the LDF file
* schedule_file - same as ldf_file
* schedule_table_name - which schduler you want to use when you are running the device as master
* schedule_autostart - master: true, slave: false
```json 
{
      "namespace": "LinSlave",
      "type": "lin",
      "config": {
        "device_identifier": 1,
        "server_port": 2014,
        "target_host": null,
        "target_port": 2013
      },
      "device_name": "lin",
      "node_mode": "slave",
      "ldf_file": "configuration/ldf_files/linone.ldf",
      "schedule_file": "configuration/ldf_files/linone.ldf",
      "schedule_table_name": "linoneSchedule",
      "schedule_autostart": false
}
```

### Debug

Logging is by default output to serial 
```
constexpr bool LOG_TO_SERIAL = true;
```
To output logs in signalbroker do
```
constexpr bool LOG_TO_SERIAL = false;
```