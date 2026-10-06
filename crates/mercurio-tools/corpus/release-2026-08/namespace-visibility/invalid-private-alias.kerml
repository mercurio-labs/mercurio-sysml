package P { class A; private alias Secret for A; }
package Q { private import P::*; feature x: Secret; }
