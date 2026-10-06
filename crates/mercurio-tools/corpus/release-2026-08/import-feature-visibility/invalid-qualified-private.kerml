package P { feature Visible; } package Q { private import P::Visible; } package R { feature x :> Q::Visible; }
