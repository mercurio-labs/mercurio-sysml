package P { feature Visible; } package Q { public import P::Visible; } package R { private import Q::*; feature x :> Visible; }
