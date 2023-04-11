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
            commands::init();
        }
    } else if args[0] == "cat-file" {
        if args.len() == 3 && args[1] == "-p" {
            commands::cat_file_print(&args[2]).unwrap();
        } else {
            println!("Unrecognised 'cat-file' signature!");
        }
    } else if args[0] == "hash-object" {
        if args.len() == 3 && args[1] == "-w" {
            commands::hash_object_write(&args[2]).unwrap();
        } else {
            println!("Unrecognised 'hash-object' signature!");
        }
    } else if args[0] == "ls-tree" {
        if args.len() == 3 && args[1] == "--name-only" {
            commands::read_tree_names(&args[2]).unwrap();
        } else {
            println!("Unrecognised 'ls-tree' signature!");
        }
    } else {
        println!("unknown command: {}", args[0]);
    }
}