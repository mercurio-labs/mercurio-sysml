package P { class A { feature original; private alias Hidden for original; } class B :> A { feature b :> Hidden; } }
