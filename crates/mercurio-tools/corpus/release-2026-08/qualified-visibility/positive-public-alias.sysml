package P { private part def Hidden; public alias Visible for Hidden; } package Q { part x: P::Visible; }
