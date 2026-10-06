package InterfaceEnds {
    port portA;
    port portB;
    port portC;
    interface namedBinary connect left ::> portA to right ::> portB;
    interface anonymousBinary connect portA to portB;
    interface namedNary connect (alpha ::> portA, beta ::> portB, gamma ::> portC);
    interface anonymousNary connect (portA, portB, portC);
    connection genericConnection connect portA to portB;
}
