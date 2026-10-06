package P { class A { feature original; protected alias Hidden for original; } class B :> A { feature b :> Hidden; } }
