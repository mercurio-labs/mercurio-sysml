package PortTypeConnections {
    private import PortTypeSources::*;
    alias choice for supplied;
    interface typed connect left ::> choice to right ::> multi;
    interface path connect host.outlet to supplied;
    interface mixedNamed connect left ::> supplied to multi;
    interface mixedAnonymous connect supplied to right ::> multi;
    interface minimal connect redundant to supplied;
    connection genericNamed connect left ::> supplied to multi;
    connection genericAnonymous connect supplied to right ::> multi;
}
