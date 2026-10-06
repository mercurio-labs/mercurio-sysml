package P { private class Hidden; public alias Public for Hidden; }
package Q { private import P::*; feature x: Public; }
