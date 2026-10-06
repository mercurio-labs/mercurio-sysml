package P { class Visible; } package Q { public import P::Visible; } package R { class B :> Q::Visible; }
