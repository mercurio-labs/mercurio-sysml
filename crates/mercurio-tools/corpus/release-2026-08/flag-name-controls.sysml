package FlagNameControls {
    part def <veh> Vehicle;
    part def Person;
    requirement def <req> Need {
        subject system : Vehicle;
        stakeholder reviewer : Person;
    }
    use case def Drive { subject car : Vehicle; actor driver : Person; }
    item def Cart;
    item def Product;
    connection def Selection {
        end inCart [0..1] item cart : Cart;
        end products [0..*] item product : Product;
    }
    port def DrivePort;
    part host {
        port left : DrivePort;
        port right : DrivePort;
        interface shaft connect source ::> left to target ::> right;
        attribute flag : ScalarValues::Boolean;
        action behavior {
            action change assign host.flag := true;
            loop action cycle { } until true;
        }
        action sense;
        state running { do sense { out temp; } }
    }
}
