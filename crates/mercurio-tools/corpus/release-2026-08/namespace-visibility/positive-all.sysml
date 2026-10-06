package P { private part def Hidden; private alias Secret for Hidden; }
package Q { private import all P::*; part x: Secret; }
