package VehicleInterfaceExample {
  port def PowerPort;
  part def BatteryModule;
  part def DriveController;

  part def Vehicle {
    part battery : BatteryModule;
    part controller : DriveController;
    port power : PowerPort;

    connect controller to power;
  }
}
