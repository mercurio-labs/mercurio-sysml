package P { private package Inner { class Hidden; } }
package Q { private import P::*::**; feature x: Hidden; }
