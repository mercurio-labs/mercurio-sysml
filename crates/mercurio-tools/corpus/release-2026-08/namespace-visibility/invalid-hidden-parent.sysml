package P { private package Inner { part def Hidden; } }
package Q { private import P::*::**; part x: Hidden; }
