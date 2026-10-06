package ReleaseUsageFlags {
    attribute topAttribute;
    attribute sequence[*] ordered nonunique;
    action topAction;
    part def Host {
        attribute varying;
        constant attribute stable;
        action composed;
        ref action observed;
        in action parameter;
        port socket;
        timeslice part portion;
        part nested { attribute nestedValue; }
    }
    attribute def Data { attribute field; }
    action def Behavior { attribute value; }
    port def Bus { port nestedPort; }
}
