use std::env;
use git_starter_rust::commands;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        println!("git called with an empty argument list!");
        return;
    }

    if args[0] == "init" {
        if args.len() != 1 {
            println!("Unrecognised 'init' signature!");
        } else {
            println!("{}", commands::init());
        }
    } else if args[0] == "cat-file" {
        if args.len() == 3 && args[1] == "-p" {
            match commands::cat_file_print(&args[2]) {
                Ok(r) => print!("{r}"),
                Err(err) => println!("{}", err.to_string()),
            }
        } else {
            println!("Unrecognised 'cat-file' signature!");
        }
    } else if args[0] == "hash-object" {
        if args.len() == 3 && args[1] == "-w" {
            match commands::hash_object_write(&args[2]) {
                Ok(r) => println!("{r}"),
                Err(err) => println!("{}", err.to_string()),
            }
        } else {
            println!("Unrecognised 'hash-object' signature!");
        }
    } else if args[0] == "ls-tree" {
        if args.len() == 3 && args[1] == "--name-only" {
            match commands::read_tree_names(&args[2]) {
                Ok(r) => println!("{r}"),
                Err(err) => println!("{}", err.to_string()),
            }
        } else {
            println!("Unrecognised 'ls-tree' signature!");
        }
    } else {
        println!("unknown command: {}", args[0]);
    }
}