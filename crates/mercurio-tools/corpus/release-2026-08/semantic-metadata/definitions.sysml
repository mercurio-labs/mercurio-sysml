package MetadataBases {
    part def Device { part inheritedMember; }
    part devices : Device;
    part def Other;
    part others : Other;
    metadata def <device> DeviceTag :> Metaobjects::SemanticMetadata {
        :>> baseType default devices meta SysML::PartUsage;
    }
    metadata def Inherited :> DeviceTag;
    metadata def Override :> Inherited {
        :>> baseType = others meta SysML::PartUsage;
    }
    metadata def ClassOnly :> Metaobjects::SemanticMetadata {
        :>> baseType = Device meta SysML::PartDefinition;
    }
    metadata def Plain;
}
