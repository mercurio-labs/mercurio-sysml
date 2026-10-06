package P { feature Visible; } package Q { public import P::Visible; } package R { feature x :> Q::Visible; }
