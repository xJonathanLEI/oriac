/// A trace entry for every instruction that was executed. Holds the register values before the
/// instruction was executed.
#[derive(Debug)]
pub struct TraceEntry<T> {
    pub pc: T,
    pub ap: T,
    pub fp: T,
}
