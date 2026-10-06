package Classifiers {
    class Base;
    type Specialized :> Base { feature contained : Base; }
    type MoreSpecialized :> Specialized;
    type Conjugate ~ Specialized;
    type Unioned :> Base unions Specialized, MoreSpecialized;
    assoc struct Pair { end feature left : Base; end feature right : Base; }
}
