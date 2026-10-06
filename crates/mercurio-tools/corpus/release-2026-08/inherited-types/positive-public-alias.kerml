package P { class A { private class Hidden; public alias Visible for Hidden; } class B :> A { class Derived :> Visible; feature b : Visible; } }
