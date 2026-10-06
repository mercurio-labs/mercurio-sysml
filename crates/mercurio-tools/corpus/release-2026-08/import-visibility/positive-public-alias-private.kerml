package P { private class Hidden; public alias Visible for Hidden; } package Q { private import P::Visible; class B :> Visible; }
