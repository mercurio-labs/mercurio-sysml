package P { part def A; private alias Secret for A; }
package Q { private import P::*; part x: Secret; }
