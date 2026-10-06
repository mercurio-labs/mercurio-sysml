package P { class A { private feature Hidden; public alias Visible for Hidden; } class B :> A { feature b :> Visible; } }
