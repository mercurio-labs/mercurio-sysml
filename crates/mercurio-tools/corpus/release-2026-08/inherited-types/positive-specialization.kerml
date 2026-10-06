package P { class A { protected class Hidden { feature original; } } class B :> A { class Derived :> Hidden { feature b :> original; } } }
