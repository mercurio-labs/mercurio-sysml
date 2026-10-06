package P { private part def Hidden; public alias Public for Hidden; }
package Q { private import P::*; part x: Public; }
