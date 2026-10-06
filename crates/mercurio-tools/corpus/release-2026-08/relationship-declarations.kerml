package RelationshipDeclarations {
    classifier A;
    classifier B;
    feature f;
    feature g;
    class Parent { feature x; }
    class Child :> Parent { feature y; }
    type Original specializes Base::Anything;
    type Conjugated specializes Base::Anything;
    specialization <sc> Super subclassifier RelationshipDeclarations::B specializes RelationshipDeclarations::A;
    specialization <sp> Gen subtype RelationshipDeclarations::B :> RelationshipDeclarations::A;
    specialization <ft> Typing typing RelationshipDeclarations::f typed by RelationshipDeclarations::A;
    specialization <ss> Sub subset RelationshipDeclarations::f subsets RelationshipDeclarations::g;
    specialization <rd> Redef redefinition RelationshipDeclarations::Child::y redefines RelationshipDeclarations::Parent::x;
    conjugation <cj> Conj conjugate RelationshipDeclarations::Conjugated ~ RelationshipDeclarations::Original;
    inverting <iv> Invert inverse RelationshipDeclarations::f of RelationshipDeclarations::g;
    featuring <tf> Featuring of RelationshipDeclarations::f by RelationshipDeclarations::A;
    dependency <dp> Use from RelationshipDeclarations::A, RelationshipDeclarations::f to RelationshipDeclarations::B, RelationshipDeclarations::g;
    subclassifier B :> A;
    subtype B specializes A;
    typing g : A;
    subset f :> g;
    redefinition Child::y :>> Parent::x;
    conjugate Conjugated conjugates Original;
    inverse f of g;
    featuring g by A;
    dependency B, g to A, f;
}
