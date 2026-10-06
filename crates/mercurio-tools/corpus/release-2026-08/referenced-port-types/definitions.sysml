package PortTypeDefinitions {
    port def Base;
    port def Derived :> Base;
    port def Other;
    part def Owner { port outlet : Derived; }
    part def OwnerExtension :> Owner;
}
