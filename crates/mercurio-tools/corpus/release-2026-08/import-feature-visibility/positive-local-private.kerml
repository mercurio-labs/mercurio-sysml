package P { feature Visible; } package Q { private import P::Visible; feature x :> Visible; }
