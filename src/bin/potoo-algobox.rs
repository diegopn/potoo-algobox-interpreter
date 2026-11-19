use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use potoo_algobox_interpreter::{Host, Interpreter, Value, parse};

struct CliHost {
    state: u64,
}
impl Host for CliHost {
    fn input(&mut self) -> Result<String, String> {
        let mut text = String::new();
        io::stdin()
            .read_line(&mut text)
            .map_err(|e| e.to_string())?;
        Ok(text.trim_end_matches(['\r', '\n']).to_owned())
    }
    fn display(&mut self, value: &Value, newline: bool) -> Result<(), String> {
        if newline {
            println!("{value}");
        } else {
            print!("{value}");
            io::stdout().flush().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    fn confirm_continue_after_output_limit(&mut self) -> Result<bool, String> {
        print!("1 000 affichages produits. Continuer ? [c/N] ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        let mut response = String::new();
        io::stdin()
            .read_line(&mut response)
            .map_err(|e| e.to_string())?;
        Ok(matches!(
            response.trim().to_ascii_lowercase().as_str(),
            "c" | "continuer"
        ))
    }
    fn pause(&mut self) -> Result<(), String> {
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    fn draw(&mut self, command: &str, _args: &[Value]) -> Result<(), String> {
        Err(format!("{command} requires a graphical host"))
    }
    fn random_integer(&mut self, lower: i64, upper: i64) -> Result<i64, String> {
        if lower > upper {
            return Err("lower bound exceeds upper bound".into());
        }
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        let width = (upper as i128 - lower as i128 + 1) as u128;
        Ok((lower as i128 + (self.state as u128 % width) as i128) as i64)
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), String> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| "usage: potoo-algobox <source-file>".to_owned())?;
    let source = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    let program = parse(&source).map_err(|e| {
        e.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos() as u64
        | 1;
    Interpreter::new(CliHost { state: seed })
        .run(&program)
        .map_err(|e| e.to_string())?;
    Ok(())
}
