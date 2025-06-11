use clap_mangen::Man;
use std::fs::File;
use std::path::Path;

use terradrift::cli::Cli;
use clap::CommandFactory;

fn main() {
    let out_path = std::env::args().nth(1).unwrap_or_else(|| "terradrift.1".to_string());
    let cmd = Cli::command();
    let man = Man::new(cmd);
    let path = Path::new(&out_path);
    let mut file = File::create(path).expect("create man page");
    man.render(&mut file).expect("render man page");
    eprintln!("Generated man page at {}", path.display());
} 