package P { class Visible; } package Q { private import P::Visible; } package R { class B :> Q::Visible; }
