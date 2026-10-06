package P { class Visible; } package Q { private import P::Visible; } package R { private import Q::*; class B :> Visible; }
