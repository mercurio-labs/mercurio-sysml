package P { private feature Hidden; } package Q { private import all P::*; feature x :> Hidden; }
