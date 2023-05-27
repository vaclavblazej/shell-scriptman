use clap::{arg, command, Command};
use anyhow::Result;
use std::{io::Write, path::PathBuf, process::ExitStatus};
use serde_derive::{Serialize, Deserialize};
use std::os::unix::fs::PermissionsExt;

fn execute(in_dir: Option<PathBuf>, cmd: &String, args: impl IntoIterator<Item = String>) -> Result<ExitStatus, std::io::Error> {
    if let Some(dir) = in_dir {
        std::env::set_current_dir(dir)?;
    }
    std::process::Command::new(cmd)
        .args(args)
        .spawn()
        .expect(format!("ERROR: Failed to execute command {cmd}").as_str())
        .wait()
}

#[derive(Serialize, Deserialize, Clone)]
struct JsonCmd {
    alias: String,
    rel_path: String,
    description: String,
}

impl JsonCmd {
    fn to_cmd(&self, scope: &Scope, scope_path: &PathBuf) -> Cmd {
        Cmd{
            alias: self.alias.to_owned(),
            rel_path: self.rel_path.to_owned(),
            description: self.description.to_owned(),
            abs_path: path_to_str(&scope_path.join(self.rel_path.to_owned())),
            scope: scope.to_owned(),
            scope_path: scope_path.to_owned(),
        }
    }
    fn to_cmd_with_group(&self, group: &CmdGroup) -> Cmd {
        self.to_cmd(&group.scope, &group.scope_path)
    }
}

#[derive(Clone)]
struct Cmd {
    alias: String,
    rel_path: String,
    description: String,
    abs_path: String,
    scope: Scope,
    scope_path: PathBuf,
}

impl Cmd {
    fn to_json_cmd(&self) -> JsonCmd {
        JsonCmd{
            alias: self.alias.to_owned(),
            rel_path: self.rel_path.to_owned(),
            description: self.description.to_owned(),
        }
    }
}

#[derive(Clone)]
struct CmdGroup {
    commands: Vec<Cmd>,
    scope: Scope,
    scope_path: PathBuf,
}

impl CmdGroup {
    fn new(scope: &Scope, scope_path: &PathBuf) -> Result<CmdGroup> {
        let command_path = scope_path.join(".cmd").join("index.json").to_owned();
        let commands = from_file(&command_path)?.into_iter().map(|c|c.to_cmd(scope, scope_path)).collect();
        Ok(CmdGroup{
            commands,
            scope: scope.to_owned(),
            scope_path: scope_path.to_owned(),
        })
    }
}

fn to_file(path: &PathBuf, cmd_group: &CmdGroup) -> Result<(), std::io::Error> {
    let commands = &cmd_group.commands;
    let json_commands: Vec<JsonCmd> = commands.into_iter().map(|c|c.to_json_cmd()).collect();
    let data = serde_json::to_string_pretty(&json_commands)?;
    std::fs::write(path, data)
}

fn from_file(path: &PathBuf) -> Result<Vec<JsonCmd>> { // todo consider explicit errors
    let data = std::fs::read_to_string(path)?;
    let commands = serde_json::from_str::<Vec<JsonCmd>>(&data)?;
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

fn path_to_str(path: &PathBuf) -> String {
    path.to_owned().into_os_string().into_string().expect("unable to convert path to string")
}

#[derive(PartialEq, Clone)]
enum Scope {
    GLOBAL,
    PROJECT,
}

impl Scope{
    fn to_path(&self, global_path: &PathBuf, project_path: &Option<PathBuf>) -> PathBuf{
        return match self {
            Scope::GLOBAL => global_path.to_owned(),
            Scope::PROJECT => project_path.to_owned().unwrap(),
        };
    }
}

fn get_scope(global: bool, project: bool, project_path: &Option<PathBuf>) -> Scope {
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
        Err(e) => panic!("cannot retrieve directory of the executable -- place for the global scope scripts: {e}"),
    }
}

fn ensure_initialized(path: &PathBuf, report: bool) -> Result<PathBuf> {
    let cmd_dir = path.join(".cmd");
    if let Err(_) = std::fs::create_dir(&cmd_dir) {
        if report { println!("INFO: ./.cmd/ folder already exists"); }
    }
    let cmd_subdir = cmd_dir.join("commands");
    if let Err(_) = std::fs::create_dir(&cmd_subdir) {
        if report { println!("INFO: ./.cmd/commands/ folder already exists"); }
    }
    let file_path = cmd_dir.join("index.json");
    if file_path.exists() {
        if report { println!("INFO ./.cmd/index.json file already exists"); }
        return Ok(file_path);
    }
    if let Ok(mut file) = std::fs::File::create(&file_path){
        file.write_all(b"[]")?;
    } else {
        if report { println!("INFO: ./.cmd/index.json file already exists"); }
    }
    Ok(file_path)
}

fn cmd_init_project() -> Result<()> {
    let current_dir: PathBuf = std::env::current_dir().expect("unable to retrieve current directory");
    ensure_initialized(&current_dir, true)?;
    Ok(())
}

fn cmd_add(dir: &PathBuf, command: &JsonCmd, cmd_group: &mut CmdGroup) -> Result<()> {
    let script_path = dir.join(&command.rel_path);
    let commands_file = ensure_initialized(dir, false)?;
    if !script_path.exists() {
        let mut file = std::fs::File::create(&script_path).expect("unable to create file");
        file.write_all(b"#!/usr/bin/env sh\n\necho \"Hello world\"\n")?;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o775)).expect("unable to assign script permissions");
    }
    cmd_group.commands.push(command.to_cmd_with_group(&cmd_group));
    to_file(&commands_file, cmd_group)?;
    cmd_edit(&script_path)?;
    Ok(())
}

fn cmd_edit(script_path: &PathBuf) -> Result<ExitStatus, std::io::Error> {
    let editor = std::env::var("EDITOR").unwrap_or("vim".into());
    let f: String = path_to_str(script_path);
    execute(None, &editor, [f])
}

// fn cmd_remove(command_str: &String, groups: &mut Vec<(CmdGroup, Scope)>) -> Result<()> {
    // if let Some((command, scope)) = find_command(groups, command_str) {
        // let commands_file = ensure_initialized(&command.scope_path, false)?;
        // for (group, scope) in groups {
            // if *scope == command.scope {
                // group.commands = group.commands.into_iter().filter(|c|{
                    // c.alias != command_str
                // }).collect();
                // return Ok(());
            // }
        // }
        // to_file(&commands_file, groups)?;
    // }
    // println!("command {command_str} not found");
    // Ok(())
// }


fn find_command<'a>(groups: &'a Vec<(CmdGroup, Scope)>, pattern: &String) -> Option<(&'a Cmd, Scope)> {
    for (group, scope) in groups {
        for command in &group.commands {
            if command.alias == *pattern {
                return Some((command, scope.to_owned()));
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
                     Command::new("--init").visible_alias("-i")
                     .about("Setup project scope in the current directory"),
                     Command::new("--add").visible_alias("-a")
                     .arg(arg!(<ALIAS>))
                     .about("Create script and open it in the $EDITOR"),
                     Command::new("--edit").visible_alias("-e")
                     .arg(arg!([ALIAS]).value_parser(clap::value_parser!(String)))
                     .about("Open script index or [SCRIPT] in the $EDITOR"),
                     Command::new("--remove").visible_alias("-r")
                     .arg(arg!(<ALIAS>))
                     .about("Remove script from the index (does NOT remove file)"),
                     Command::new("--version")
                     .about("Prints out version information")
        ])
        .args([
              arg!(-p --project "Force project scope"),
              arg!(-g --global "Force global scope"),
        ].map(|x|x.required(false)))
        ;
    let mut cmd_groups: Vec<(CmdGroup, Scope)> = vec![];
    let global_path = find_global_dir();
    if let Ok(global) = CmdGroup::new(&Scope::GLOBAL, &global_path) {
        cmd_groups.push((global.to_owned(), Scope::GLOBAL));
    }
    let mut project_commands: Option<CmdGroup> = None;
    let project_path = find_project_dir();
    if let Some(project_dir) = &project_path {
        match CmdGroup::new(&Scope::PROJECT, &project_dir) {
            Ok(commands) => project_commands = Some(commands),
            Err(e) => println!("ERR: {:?}", e),
        }
    }
    if let Some(project_commands) = &project_commands {
        cmd_groups.push((project_commands.to_owned(), Scope::PROJECT));
    }
    for (group, _) in &cmd_groups {
        for command in &group.commands {
            builder = builder.subcommand(
                Command::new(&command.alias)
                .about(&command.description)
                .arg(arg!([args]...))
                );
        }
    }
    let cli_args = builder.get_matches_mut();
    let scope = get_scope(
        cli_args.get_flag("project"),
        cli_args.get_flag("global"),
        &project_path,
        );
    let path = scope.to_path(&global_path, &project_path);
    let (subcommand, matched_args) = match cli_args.subcommand() {
        Some((subcommand, matched_args)) => (subcommand, matched_args),
        None => return Ok(()),
    };
    match subcommand {
        "--init"|"-i" => {
            cmd_init_project().expect("cannot initialize project");
        },
        "--add"|"-a" => {
            let mut args = matched_args.get_many::<String>("ALIAS").unwrap().map(|s| s.to_string());
            let alias: String = args.next().unwrap();
            let description: String = args.next().unwrap_or("".to_string());
            match find_command(&cmd_groups, &alias) {
                Some(_) => {
                    panic!("Unable to create {alias} because it already exists");
                },
                None => {
                    let rel_path = format!("./.cmd/commands/{alias}.sh");
                    let command = JsonCmd { alias, rel_path, description };
                    let mut group: CmdGroup = match scope {
                        Scope::GLOBAL => CmdGroup::new(&Scope::PROJECT, &global_path)?,
                        Scope::PROJECT => {
                            if let Some(project_commands) = &project_commands {
                                project_commands.to_owned()
                            } else {
                                CmdGroup::new(&Scope::PROJECT, &global_path)?
                            }
                        },
                    };
                    cmd_add(&path, &command, &mut group).expect("cannot add command");
                },
            }
        },
        "--edit"|"-e" => {
            let some_alias = matched_args.get_one::<String>("ALIAS");
            if let Some(alias) = some_alias{
                if let Some((command, _)) = find_command(&cmd_groups, &alias) {
                    cmd_edit(&path.join(&command.rel_path))?;
                } else {
                    println!("{alias} is an unknown command");
                }
            } else {
                cmd_edit(&path.join(".cmd").join("index.json"))?;
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
                Some((command, scope)) => {
                    let script_path = Some(scope.to_path(&global_path, &project_path));
                    execute(script_path, &command.rel_path, args)?;
                },
                None => {
                    panic!("unknown subcommand returned from parser");
                }
            }
        },
    }
    Ok(())
}
