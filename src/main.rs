use std::env;
use git_starter_rust::commands;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args[0] == "init" {
        if args.len() != 1 {
            println!("Unrecognised 'init' signature!");
        } else {
            commands::init();
        }
    } else if args[0] == "cat-file" {
        if args.len() == 3 && args[1] == "-p" {
            commands::cat_file_pretty(&args[2]).unwrap();
        } else {
            println!("Unrecognised 'cat-file' signature!");
        }
    } else {
        println!("unknown command: {}", args[0]);
    }
}