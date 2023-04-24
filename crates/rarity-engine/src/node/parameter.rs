#[derive(Clone, Debug)]
pub struct Parameter {
    pub addr: Vec<String>,
    pub range: ParaRange,
}

#[derive(Clone, Debug)]
pub enum ParaRange {
    Float(FloatRange),
    Enum(EnumRange),
}

#[derive(Clone, Debug)]
pub struct FloatRange {
    pub name: String,
    pub min: f64,
    pub max: f64,
    pub default: f64,
}

#[derive(Clone, Debug)]
pub struct EnumRange {
    pub name: String,
    pub len: usize,
    pub default: usize,
}
