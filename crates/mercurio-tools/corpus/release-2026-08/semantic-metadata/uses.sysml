package MetadataUses {
    private import MetadataBases::*;
    #device def Extended { #Inherited nested; }
    #Inherited usage { #Inherited nested; }
    #Override def Overridden;
    #Override overridden;
    #ClassOnly def Direct;
    #ClassOnly classifierOnUsage;
    #Plain def UntypedDefinition;
    #Plain untyped;
    #Plain #Inherited stacked;
    part def Derived :> Extended;
    part instance : Derived { ref part inheritedCopy :>> inheritedMember; }
}
