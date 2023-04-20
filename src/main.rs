use std::env;
use git_starter_rust::commands;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        println!("git called with an empty argument list!");
        return;
    }

    match args[0].as_str() {
        "init" => {
            if args.len() == 1 {
                println!("{}", commands::init());
            } else {
                println!("Unrecognised 'init' signature!");
            }
        }
        "cat-file" => {
            if args.len() == 3 && args[1] == "-p" {
                match commands::cat_file_print(&args[2]) {
                    Ok(r) => print!("{r}"),
                    Err(err) => println!("{}", err),
                }
            } else {
                println!("Unrecognised 'cat-file' signature!");
            }
        }
        "hash-object" => {
            if args.len() == 3 && args[1] == "-w" {
                match commands::hash_object_write(&args[2]) {
                    Ok(r) => println!("{r}"),
                    Err(err) => println!("{}", err),
                }
            } else {
                println!("Unrecognised 'hash-object' signature!");
            }
        }
        "ls-tree" => {
            if args.len() == 3 && args[1] == "--name-only" {
                match commands::read_tree_names(&args[2]) {
                    Ok(r) => println!("{r}"),
                    Err(err) => println!("{}", err),
                }
            } else {
                println!("Unrecognised 'ls-tree' signature!");
            }
        }
        "write-tree" => {
            if args.len() == 1 {
                match commands::write_tree() {
                    Ok(r) => println!("{r}"),
                    Err(err) => println!("{}", err),
                }
            } else {
                println!("Unrecognised 'write-tree' signature!");
            }
        }
        "commit-tree" => {
            if args.len() == 6 && args[2] == "-p" && args[4] == "-m" {
                match commands::create_commit_with_message(&args[1], &args[3], &args[5]) {
                    Ok(r) => println!("{r}"),
                    Err(err) => println!("{}", err),
                }
            } else {
                println!("Unrecognised 'commit-tree' signature!");
            }
        }
        "clone" => {
            if args.len() == 3 {
                match commands::clone_repo(&args[1], &args[2]) {
                    Ok(r) => println!("{r}"),
                    Err(err) => println!("{}", err),
                }
            } else {
                println!("Unrecognised 'clone' signature!");
            }
        }
        _ => {
            println!("unknown command: {}", args[0]);
        }
    }
}