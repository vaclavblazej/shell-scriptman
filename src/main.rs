use clap::{arg, command, Command};
use anyhow::Result;
use std::{io::Write, path::PathBuf, process::ExitStatus};
use serde_derive::{Serialize, Deserialize};

fn run_script(cmd: &String, args: impl IntoIterator<Item = String>) -> Result<ExitStatus, std::io::Error> {
    std::process::Command::new(cmd)
        .args(args)
        .spawn()
        .expect("ERROR: Failed to execute command")
        .wait()
}

#[derive(Serialize, Deserialize, Clone)]
struct Cmd {
    alias: String,
    rel_path: String,
    description: String,
}

#[derive(Clone)]
struct CmdGroup { // todo consider removing
    commands: Vec<Cmd>,
}

fn to_file(path: &PathBuf, cmd_group: &CmdGroup) -> Result<(), std::io::Error> {
    let data = serde_json::to_string_pretty(&cmd_group.commands)?;
    std::fs::write(path, data)
}

fn from_file(path: &PathBuf) -> Result<Vec<Cmd>> { // todo consider explicit errors
    let data = std::fs::read_to_string(path)?;
    let commands = serde_json::from_str::<Vec<Cmd>>(&data)?;
    Ok(commands)
}

fn find_project_dir() -> Option<PathBuf> {
    let mut dir: PathBuf = std::env::current_dir().unwrap();
    loop {
        if dir.join(".cmd").exists() {
            return Some(dir.to_path_buf());
        }
        if !dir.pop() {
            return None;
        }
    }
}

enum Scope {
    GLOBAL,
    PROJECT,
}

fn get_mode(global: bool, project: bool, project_path: &Option<PathBuf>) -> Scope {
    if global {
        Scope::GLOBAL
    } else {
        match project_path {
            Some(_) => Scope::PROJECT,
            None => {
                if project {
                    panic!("Project option forced but no project is initialized");
                } else {
                    Scope::GLOBAL
                }
            },
        }
    }
}

fn find_global_dir() -> PathBuf {
    match std::env::current_exe() {
        Ok(dir) => dir,
        Err(e) => panic!("Cannot retrieve directory of the executable -- place for the global scope scripts: {e}"),
    }
}

fn load_global_commands(global_dir: &PathBuf) -> Result<CmdGroup>{
    let commands = from_file(&global_dir.join(".cmd").join("commands.json"))?;
    Ok(CmdGroup{commands})
}

fn load_project_commands(project_dir: &PathBuf) -> Result<CmdGroup>{
    let commands = from_file(&project_dir.join(".cmd").join("commands.json"))?;
    Ok(CmdGroup{commands})
}

fn ensure_initialized(path: &PathBuf) -> Result<()> {
    let cmd_dir = path.join(".cmd");
    if let Err(_) = std::fs::create_dir(&cmd_dir) {
        println!("INFO: .cmd folder already exists");
    }
    if let Ok(mut file) = std::fs::File::create(cmd_dir.join("commands.json")){
        if let Err(e) = file.write_all(b"[]") {
            println!("unable to write into file {e}");
        }
    } else {
        println!("INFO: .cmd/commands.json file already exists");
    }
    Ok(())
}

fn cmd_init() -> Result<()> {
    let current_dir: PathBuf = std::env::current_dir().unwrap();
    ensure_initialized(&current_dir)
}

fn cmd_add(dir: &PathBuf, command: &Cmd, cmd_group: &mut CmdGroup) -> Result<()> {
    let script_path = dir.join(".cmd").join(&command.rel_path);
    let mut file = std::fs::File::create(&script_path)?;
    file.write_all(b"#!/usr/bin/env sh\n\necho \"Hello world\"\n")?;
    to_file(&dir.join(".cmd").join("commands.json"), cmd_group)?;
    cmd_edit(&script_path)?;
    Ok(())
}

fn cmd_edit(script_path: &PathBuf) -> Result<ExitStatus, std::io::Error> {
    let editor = std::env::var("EDITOR").unwrap_or("vim".into());
    let f: String = script_path.to_owned().into_os_string().into_string()
        .expect("problem stringifying path {script_path}");
    run_script(&editor, [f])
}

fn find_command<'a>(groups: &'a Vec<CmdGroup>, pattern: &String) -> Option<&'a Cmd> {
    for group in groups {
        for command in &group.commands {
            if command.alias == *pattern {
                return Some(command);
            }
        }
    }
    None
}

fn main() -> Result<()> {
    let mut builder = command!()
        .disable_help_flag(true)
        .disable_help_subcommand(true)
        .disable_version_flag(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommands([
                     Command::new("--init").alias("-i")
                     .about("Setup project scope in the current directory"),
                     Command::new("--add").alias("-a")
                     .arg(arg!(<ALIAS>))
                     .about("Create script and open it in an $EDITOR"),
                     Command::new("--edit").alias("-e")
                     .arg(arg!([ALIAS]).value_parser(clap::value_parser!(String)))
                     .about("Open script index or [SCRIPT] in an $EDITOR"),
                     Command::new("--remove").alias("-r")
                     .arg(arg!(<ALIAS>))
                     .about("Remove a script"),
                     Command::new("--version")
                     .about("Prints out version information")
        ])
        .args([
              arg!(-p --project "Force project scope"),
              arg!(-g --global "Force global scope"),
        ].map(|x|x.required(false)))
        ;
    let mut cmd_groups: Vec<CmdGroup> = vec![];
    let global_path = find_global_dir();
    if let Ok(global) = load_global_commands(&global_path) {
        cmd_groups.push(global.to_owned());
    }
    let mut project_commands: Option<CmdGroup> = None;
    let project_path = find_project_dir();
    if let Some(project_dir) = &project_path {
        match load_project_commands(&project_dir) {
            Ok(commands) => project_commands = Some(commands),
            Err(e) => println!("ERR: {:?}", e),
        }
    }
    if let Some(project_commands) = &project_commands {
        cmd_groups.push(project_commands.to_owned());
    }
    for group in &cmd_groups {
        for command in &group.commands {
            builder = builder.subcommand(
                Command::new(&command.alias)
                .about(&command.description)
                .arg(arg!([args]...))
                );
        }
    }
    let cli_args = builder.get_matches_mut();
    let scope = get_mode(
        cli_args.get_flag("project"),
        cli_args.get_flag("global"),
        &project_path,
        );
    let path = match scope {
        Scope::GLOBAL => global_path.to_owned(),
        Scope::PROJECT => project_path.unwrap(),
    };
    let (subcommand, matched_args) = match cli_args.subcommand() {
        Some((subcommand, matched_args)) => (subcommand, matched_args),
        None => return Ok(()),
    };
    match subcommand {
        "--init"|"-i" => {
            cmd_init().expect("Cannot initialize project");
        },
        "--add"|"-a" => {
            let mut args = matched_args.get_many::<String>("ALIAS").unwrap().map(|s| s.to_string());
            let alias: String = args.next().unwrap();
            let description: String = args.next().unwrap();
            match find_command(&cmd_groups, &alias) {
                Some(_) => {
                    panic!("Unable to create {alias} because it already exists");
                },
                None => {
                    let script_path = path.join(".cmd").join(alias.to_owned() + ".sh");
                    let command = Cmd {
                        alias,
                        rel_path: script_path.to_str().unwrap().into(),
                        description,
                    };
                    let mut group: CmdGroup = match scope {
                        Scope::GLOBAL => load_global_commands(&global_path)?,
                        Scope::PROJECT => {
                            if let Some(project_commands) = &project_commands {
                                project_commands.to_owned()
                            } else {
                                load_global_commands(&global_path)?
                            }
                        },
                    };
                    cmd_add(&path, &command, &mut group).expect("Cannot add command");
                },
            }
        },
        "--edit"|"-e" => {
            let some_alias = matched_args.get_one::<String>("ALIAS");
            if let Some(alias) = some_alias{
                if let Some(command) = find_command(&cmd_groups, &alias) {
                    cmd_edit(&path.join(&command.rel_path))?;
                } else {
                    println!("{alias} is an unknown command");
                }
            } else {
                cmd_edit(&path.join(".cmd").join("commands.json"))?;
            }
        },
        "--remove"|"-r" => {
            panic!("Not implemented yet");  // todo
        },
        "--version" => {
            print!("{}", builder.render_version());
        },
        _ => {
            let args = match matched_args.get_many::<String>("args") {
                Some(s) => s.into_iter().map(|s| s.to_string()).collect(),
                None => vec![],
            };
            match find_command(&cmd_groups, &(*subcommand).into()) {
                Some(command) => {
                    run_script(&command.rel_path, args)?; // todo absolute path
                },
                None => {
                    panic!("unknown subcommand returned from parser");
                }
            }
        },
    }
    Ok(())
}
