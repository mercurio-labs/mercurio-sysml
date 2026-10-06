package P { private class Hidden; } package Q { private import P::Hidden; class B :> Hidden; }
