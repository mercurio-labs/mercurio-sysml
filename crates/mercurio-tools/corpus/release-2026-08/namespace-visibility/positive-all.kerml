package P { private class Hidden; private alias Secret for Hidden; }
package Q { private import all P::*; feature x: Secret; }
