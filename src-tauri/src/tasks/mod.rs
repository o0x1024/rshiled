pub mod script;

#[derive(Debug)]
pub enum Param {
    Int(i32),
    Float(f64),
    Text(String),
    Bool(bool),
}
