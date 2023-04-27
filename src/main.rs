use git_starter_rust::commands;
use clap::{ Parser, Subcommand };

#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(name = "init")] Init,
    #[command(name = "cat-file")] CatFile {
        #[clap(short = 'p')]
        pretty_print: bool,
        file_sha: String,
    },
    #[command(name = "hash-object")] HashObject {
        #[clap(short)]
        write: bool,
        file_path: String,
    },
    #[command(name = "ls-tree")] LsTree {
        #[clap(long = "name-only")]
        name_only: bool,
        tree_sha: String,
    },
    #[command(name = "write-tree")] WriteTree,
    #[command(name = "commit-tree")] CommitTree {
        tree_sha: String,
        #[clap(short)]
        parent: Option<String>,
        #[clap(short)]
        message: Option<String>,
    },
    #[command(name = "clone")] Clone {
        repo_url: String,
        folder: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init => {
            match commands::init() {
                Ok(r) => print!("{r}"),
                Err(err) => eprintln!("Error: {}", err),
            }
        }
        Commands::CatFile { pretty_print, file_sha } => {
            if *pretty_print {
                match commands::cat_file_print(file_sha) {
                    Ok(r) => print!("{r}"),
                    Err(err) => eprintln!("Error: {}", err),
                }
            } else {
                eprintln!("'cat-file' supports only print with '-p'!");
            }
        }
        Commands::HashObject { write, file_path } => {
            if *write {
                match commands::hash_object_write(file_path) {
                    Ok(r) => println!("{r}"),
                    Err(err) => eprintln!("Error: {}", err),
                }
            } else {
                eprintln!("'hash-object' supports only write with '-w'!");
            }
        }
        Commands::LsTree { name_only, tree_sha } => {
            if *name_only {
                match commands::read_tree_names(tree_sha) {
                    Ok(r) => println!("{r}"),
                    Err(err) => eprintln!("Error: {}", err),
                }
            } else {
                eprintln!("'ls-tree' supports only name print with '--name-only'!");
            }
        }
        Commands::WriteTree => {
            match commands::write_tree() {
                Ok(r) => println!("{r}"),
                Err(err) => eprintln!("Error: {}", err),
            }
        }
        Commands::CommitTree { tree_sha, parent, message } => {
            if let Some(par) = parent {
                if let Some(text) = message {
                    match commands::create_commit_with_message(tree_sha, par, text) {
                        Ok(r) => println!("{r}"),
                        Err(err) => eprintln!("Error: {}", err),
                    }
                } else {
                    match commands::create_commit_with_message(tree_sha, par, "<No message>") {
                        Ok(r) => println!("{r}"),
                        Err(err) => eprintln!("Error: {}", err),
                    }
                }
            } else {
                eprintln!("'commit-tree' needs parent provided with '-p'");
            }
        }
        Commands::Clone { repo_url, folder } => {
            if let Some(path) = folder {
                match commands::clone_repo(repo_url, &path) {
                    Ok(r) => println!("{r}"),
                    Err(err) => eprintln!("Error: {}", err),
                }
            } else if let Some((_, new_folder_name)) = repo_url.rsplit_once('/') {
                match commands::clone_repo(repo_url, &format!("./{new_folder_name}")) {
                    Ok(r) => println!("{r}"),
                    Err(err) => eprintln!("Error: {}", err),
                }
            } else {
                eprintln!("'clone' has incorrect url");
            }
        }
    }
}