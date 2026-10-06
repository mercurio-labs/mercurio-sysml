package P {
 class A;
 alias <short> Long for A {
 doc /* Alias documentation */
 comment /* Alias comment */
 rep language "text" /* Alias representation */
 }
 private alias Hidden for A;
 protected alias Guarded for A;
 alias for A;
}
