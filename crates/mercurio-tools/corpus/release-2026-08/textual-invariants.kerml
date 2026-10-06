package TextualInvariants {
    class C {
        inv plain { true }
        inv true positive { true }
        inv false negative { false }
        inv described {
            rep <ocl> inOCL language "ocl" /* self.x > 0.0 */
        }
        rep language "plain" /*  preserve spaces  */
        language "alf" /* first line
 * second line
 */
    }
}
