/// A reference to a memory address that is defined for a specific location in the program (pc).
/// The reference may be evaluated for other locations in the program, as long as its value is well
/// defined.
///
/// For example,
///
/// ```cairo
///   let x = ap   # Defines a reference to ap, that is attached to the following instruction.
///   [ap] = 5; ap++
///   # Since ap increased, the reference evaluated now should be (ap - 1), rather than ap.
///   [ap] = [x] * 2; ap++ # Thus, this instruction will translate to '[ap] = [ap - 1] * 2; ap++'
///                        # and will set [ap] to 10.
/// ```
pub struct Reference {}
