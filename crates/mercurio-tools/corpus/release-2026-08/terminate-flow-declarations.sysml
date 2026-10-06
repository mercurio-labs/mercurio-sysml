package DeclarationControls {
    item def Fuel;
    part def Scope {
        part source { item fuel : Fuel; }
        part target { item fuel : Fuel; }
        flow of fuel : Fuel from source.fuel to target.fuel;
        flow of Fuel from source.fuel to target.fuel;
    }
    action def Run {
        action worker;
        action stop terminate worker {
            attribute marker : ScalarValues::Boolean = true;
        }
        action finish terminate;
        action terminate worker;
    }
}
