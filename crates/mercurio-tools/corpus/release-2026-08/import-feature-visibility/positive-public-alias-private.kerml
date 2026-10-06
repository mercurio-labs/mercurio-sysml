package P { private feature Hidden; public alias Visible for Hidden; } package Q { private import P::Visible; feature x :> Visible; }
