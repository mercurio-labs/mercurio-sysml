package P { private class Hidden; } package Q { private import all P::*; class B :> Hidden; }
