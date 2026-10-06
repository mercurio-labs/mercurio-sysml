package P { class Visible; } package Q { public import P::Visible; } package R { private import Q::*; class B :> Visible; }
