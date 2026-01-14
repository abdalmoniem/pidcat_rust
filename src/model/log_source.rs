use std::process::Child;

#[derive(Debug)]
pub enum LogSource {
    Process(Child),
    Stdin,
}
