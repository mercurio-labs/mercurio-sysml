package VehicleRequirementExample {
  part def VehicleController;

  requirement <'REQ-START-001'> safeStart {
    doc /* Vehicle shall prevent unsafe starts. */
  }

  part controller : VehicleController {
    satisfy VehicleRequirementExample.safeStart;
  }
}
