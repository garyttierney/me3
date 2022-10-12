pub enum FieldKind {
    Fixed { signed: bool, size: usize },
    Real { size: usize },
    Bytes { size: usize },
    String { length: usize },
    WideString { length: usize },
}

pub struct Field {}

pub struct ParamDef {}
