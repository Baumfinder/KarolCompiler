mod lexer;
mod parser;

use lexer::{lexer, Token};

use clap::Parser;
use std::fs::File;
use std::io::{prelude::*, self};
use std::path::Path;

use crate::parser::AST;

#[derive(Debug, Parser)]
struct Args {
    #[arg(default_value_t=String::from("text.txt"))]
    infile: String,
    #[arg(short, long, default_value_t=String::from("out.txt"))]
    outfile: String,
    #[arg(short, long, default_value_t=50)]
    mem_size: usize,
    #[arg(long)]
    print_tokens: bool,
    #[arg(long)]
    print_ast: bool,
    #[arg(long)]
    print_code: bool,
}

fn main() {
    let args = Args::parse();
    let infile_path = Path::new(&args.infile);
    let outfile_path = Path::new(&args.outfile);

    let mut infile = match File::open(&infile_path) {
        Err(why) => panic!("Eingabedatei konnte nicht geöffnet werden: {}", why),
        Ok(file) => file,
    };
    let mut outfile = match File::create(&outfile_path) {
        Err(why) => panic!("Konnte Ausgabedatei nicht erstellen: {}", why),
        Ok(file) => file,
    };

    let mut infile_text = String::new();
    match infile.read_to_string(&mut infile_text) {
        Err(why) => panic!("Fehler beim Lesen der Eingabedatei: {}", why),
        Ok(_) => {},
    }

    // Hier passiert der shit

    let tokens: Vec<Token> = lexer(infile_text);
    if args.print_tokens {
        println!("{:#?}\n", tokens);
    }
    
    let ast: AST = parser::parse(tokens);
    if args.print_ast {
        println!("{:#?}\n", ast);
    }

    let code: String = ast.codegen();
    if args.print_code {
        println!("{}", code);
    }

    match outfile.write_all(code.as_bytes()) {
        Err(why) => panic!("Fehler beim Schreiben in die Ausgabedatei: {}", why),
        Ok(_) => {},
    }
}