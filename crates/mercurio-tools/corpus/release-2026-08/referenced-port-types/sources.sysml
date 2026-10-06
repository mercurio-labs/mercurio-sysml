package PortTypeSources {
    private import PortTypeDefinitions::*;
    port supplied : Derived;
    port multi : Derived, Other;
    port redundant : Base, Derived;
    part host : OwnerExtension;
}
