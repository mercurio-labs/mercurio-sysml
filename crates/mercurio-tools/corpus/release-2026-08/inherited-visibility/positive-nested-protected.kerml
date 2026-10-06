package P { class A { protected feature Hidden; } class B :> A { feature b { feature c :> Hidden; } } }
