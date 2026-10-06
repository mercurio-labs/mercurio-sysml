package P { feature Visible; } package Q { private import P::Visible; } package R { private import Q::*; feature x :> Visible; }
