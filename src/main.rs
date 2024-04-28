use clap::{arg, command, Command, ArgMatches, ValueHint};
use clap_complete::{generate, Generator, shells::Bash};
use anyhow::Result;
use std::{io::Write, path::PathBuf};
use serde_derive::{Serialize, Deserialize};
use std::os::unix::fs::PermissionsExt;

fn execute(cmd: &String, args: impl IntoIterator<Item = String>) {
    // if let Some(dir) = from_dir {
        // std::env::set_current_dir(dir).expect("unable to switch to folder {dir}");
    // }
    let status = std::process::Command::new(cmd)
        .args(args)
        .spawn()
        .expect(format!("ERROR: Failed to execute command {cmd}").as_str())
        .wait()
        .expect("error executing command");
    if !status.success(){
        println!("INFO: Program exited with code: {status}");
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct JsonCmd {
    alias: String,
    rel_path: String,
    description: String,
}

impl JsonCmd {
    fn to_cmd(&self, scope: &Scope) -> Cmd {
        Cmd{
            alias: self.alias.to_owned(),
            rel_path: self.rel_path.to_owned(),
            description: self.description.to_owned(),
            abs_path: scope.path.join(self.rel_path.to_owned()),
            scope: scope.to_owned(),
        }
    }
}

#[derive(Clone, Debug)]
struct Cmd {
    alias: String,
    rel_path: String,
    description: String,
    abs_path: PathBuf,
    scope: Scope,
}

impl Cmd {
    fn new(alias: &String, rel_path: &String, description: &String, scope: &Scope) -> Cmd{
        JsonCmd{
            alias: alias.to_owned(),
            rel_path: rel_path.to_owned(),
            description: description.to_owned(),
        }.to_cmd(scope)
    }
}

impl From<&Cmd> for JsonCmd {
    fn from(item: &Cmd) -> Self {
        JsonCmd{
            alias: item.alias.to_owned(),
            rel_path: item.rel_path.to_owned(),
            description: item.description.to_owned(),
        }
    }
}

#[derive(Clone, Debug)]
struct CmdGroup {
    commands: Vec<Cmd>,
    scope: Scope,
}

impl CmdGroup {
    fn new(scope: &Scope) -> Result<CmdGroup> {
        let command_path = scope.path.join(".cmd").join("index.json").to_owned();
        let commands = load_from_file(&command_path)?.into_iter().map(|c|c.to_cmd(scope)).collect();
        Ok(CmdGroup{
            commands,
            scope: scope.to_owned(),
        })
    }
}

fn save_to_file(path: &PathBuf, cmd_group: &CmdGroup) {
    let commands = &cmd_group.commands;
    let json_commands: Vec<JsonCmd> = commands.into_iter().map(|c|c.into()).collect();
    let data = serde_json::to_string_pretty(&json_commands).expect("unable to jsonify data");
    std::fs::write(path, data).expect("unable to save the index file");
}

fn load_from_file(path: &PathBuf) -> Result<Vec<JsonCmd>> {
    let data = std::fs::read_to_string(path).expect("cannot parse");
    let commands = serde_json::from_str::<Vec<JsonCmd>>(&data)?;
    Ok(commands)
}

fn find_local_dir() -> Option<PathBuf> {
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

#[derive(PartialEq, Clone, Debug)]
enum ScopeKind {
    GLOBAL,
    LOCAL,
}

#[derive(PartialEq, Clone, Debug)]
struct Scope{
    kind: ScopeKind,
    path: PathBuf,
}

fn choose_scope(cli_args: &ArgMatches, global: Scope, local: Option<Scope>) -> Scope {
    if cli_args.get_flag("global") {
        global
    } else {
        match local {
            Some(scope) => scope,
            None => {
                if cli_args.get_flag("local") {
                    panic!("local option forced but no local scope is initialized");
                }
                global
            },
        }
    }
}

fn find_global_dir() -> PathBuf {
    match std::env::current_exe() {
        Ok(mut dir) => { dir.pop(); dir }
        Err(e) => panic!("cannot retrieve directory of the executable -- place for the global scope scripts: {e}"),
    }
}

fn ensure_initialized(path: &PathBuf, report: bool) -> PathBuf {
    let cmd_dir = path.join(".cmd");
    if let Err(_) = std::fs::create_dir(&cmd_dir) {
        if report { println!("INFO: ./.cmd/ folder already exists"); }
    }
    let cmd_subdir = cmd_dir.join("scripts");
    if let Err(_) = std::fs::create_dir(&cmd_subdir) {
        if report { println!("INFO: ./.cmd/scripts/ folder already exists"); }
    }
    let file_path = cmd_dir.join("index.json");
    if file_path.exists() {
        if report { println!("INFO ./.cmd/index.json file already exists"); }
        return file_path;
    }
    if let Ok(mut file) = std::fs::File::create(&file_path){
        file.write_all(b"[]").expect("unable to write into file");
    } else {
        if report { println!("INFO: ./.cmd/index.json file already exists"); }
    }
    file_path
}

fn cmd_init_local() {
    let current_dir: PathBuf = std::env::current_dir().expect("unable to retrieve current directory");
    ensure_initialized(&current_dir, true);
}

fn cmd_add(alias: &String, description: &String, scope: &Scope, groups: &mut Vec<CmdGroup>) {
    let some_command = find_command(&alias, &groups);
    if let None = some_command {
        let rel_path = format!("./.cmd/scripts/{alias}.sh");
        let res: Option<&mut CmdGroup> = get_group_mut(&scope.kind, groups);
        if let Some(&mut ref mut group) = res{
            let command = Cmd::new(alias, &rel_path, description, &group.scope);
            let commands_file = ensure_initialized(&scope.path, false);
            if !command.abs_path.exists() {
                let mut file = std::fs::File::create(&command.abs_path).expect("unable to create file");
                file.write_all(b"#!/usr/bin/env sh\n\necho \"Hello world\"\n").expect("unable to write into file");
                std::fs::set_permissions(&command.abs_path, std::fs::Permissions::from_mode(0o775)).expect("unable to assign script permissions");
            }
            group.commands.push(command.to_owned());
            save_to_file(&commands_file, &group);
            edit_file(&command.abs_path);
        }
    } else  {
        panic!("unable to create {alias} because it already exists");
    }
}

fn cmd_edit(some_alias: Option<&String>, scope: &Scope, cmd_groups: &Vec<CmdGroup>) {
    if let Some(alias) = some_alias{
        if let Some(command) = find_command(&alias, &cmd_groups) {
            edit_file(&command.abs_path);
        } else {
            println!("{alias} is an unknown command");
        }
    } else {
        let commands_file = ensure_initialized(&scope.path, false);
        edit_file(&commands_file);
    }
}

fn edit_file(script_path: &PathBuf) {
    let editor = std::env::var("EDITOR").unwrap_or("vim".into());
    let f: String = path_to_str(script_path);
    execute(&editor, [f]);
}

fn cmd_remove(alias: &String, groups: &mut Vec<CmdGroup>) {
    if let Some(command) = find_command(&alias, &groups) {
        for group in groups {
            if group.scope == command.scope {
                let osz = group.commands.len();
                let mut res = vec![];
                std::mem::swap(&mut res, &mut group.commands);
                res = res.into_iter().filter(|c|{
                    c.alias != *command.alias
                }).collect();
                group.commands = res;
                let sz = group.commands.len();
                if sz != osz {
                    let path = group.scope.path.join(".cmd").join("index.json").to_owned();
                    save_to_file(&path, &group);
                    return;
                }
            }
        }
    } else {
        println!("{alias} is an unknown command");
    }
}

fn get_group_mut<'a>(scope_type: &ScopeKind, groups: &'a mut Vec<CmdGroup>) -> Option<&'a mut CmdGroup> {
    for group in groups {
        if group.scope.kind == *scope_type {
            return Some(group);
        }
    }
    None
}

fn find_command(alias: &String, groups: &Vec<CmdGroup>) -> Option<Cmd> {
    for group in groups {
        for command in &group.commands {
            if command.alias == *alias {
                return Some(command.to_owned());
            }
        }
    }
    None
}

fn main() {
    let mut builder = command!()
        .disable_help_flag(true)
        .disable_help_subcommand(true)
        .disable_version_flag(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommands([
                     Command::new("--init").visible_alias("-i")
                     .about("Setup local scope in the current directory"),
                     Command::new("--add").visible_alias("-a")
                     .arg(arg!(<ALIAS>).value_hint(ValueHint::Other))
                     .arg(arg!([DESCRIPTION]))
                     .about("Create script and open it in the $EDITOR"),
                     Command::new("--edit").visible_alias("-e")
                     .arg(arg!([ALIAS]).value_parser(clap::value_parser!(String)))
                     .about("Open script index or [SCRIPT] in the $EDITOR"),
                     Command::new("--remove").visible_alias("-r")
                     .arg(arg!(<ALIAS>).value_hint(ValueHint::Other))
                     .about("Remove script from the index (does NOT remove file)"),
                     Command::new("--version")
                     .about("Prints out version information"),
                     Command::new("--completions").hide(true)
        ])
        .args([
              arg!(-l --local "Force local scope"),
              arg!(-g --global "Force global scope"),
        ].map(|x|x.required(false)))
        ;
    let mut cmd_groups: Vec<CmdGroup> = vec![];
    let global_scope = Scope{kind: ScopeKind::GLOBAL, path: find_global_dir()};
    if let Ok(global) = CmdGroup::new(&global_scope) {
        cmd_groups.push(global.to_owned());
    }
    let mut local_commands: Option<CmdGroup> = None;
    let local_scope = match find_local_dir() {
        Some(local_dir) => Some(Scope{kind: ScopeKind::LOCAL, path: local_dir}),
        None => None,
    };
    if let Some(scope) = &local_scope {
        match CmdGroup::new(&scope) {
            Ok(commands) => local_commands = Some(commands),
            Err(e) => println!("ERR: {:?}", e),
        }
    }
    if let Some(local_commands) = &local_commands {
        cmd_groups.push(local_commands.to_owned());
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
    // let mut builder_copy = builder.clone();
    let cli_args = builder.get_matches_mut();
    // move to a subcommand
    // if cli_args.is_present("generate-bash-completions") {
        // generate(Bash, &mut builder_copy::build_cli(), "myapp", &mut io::stdout());
    // }
    // $ myapp generate-bash-completions > /usr/share/bash-completion/completions/myapp.bash
    let (subcommand, matched_args) = match cli_args.subcommand() {
        Some((subcommand, matched_args)) => (subcommand, matched_args),
        None => return,
    };
    match subcommand {
        "--init"|"-i" => {
            cmd_init_local();
        },
        "--add"|"-a" => {
            let alias: &String = matched_args.get_one::<String>("ALIAS").unwrap();
            let empty = "".to_string();
            let description: &String = matched_args.get_one::<String>("DESCRIPTION").unwrap_or(&empty);
            let scope = choose_scope(&cli_args, global_scope, local_scope);
            cmd_add(&alias, &description, &scope, &mut cmd_groups);
        },
        "--edit"|"-e" => {
            let some_alias = matched_args.get_one::<String>("ALIAS");
            let scope = choose_scope(&cli_args, global_scope, local_scope);
            cmd_edit(some_alias, &scope, &cmd_groups);
        },
        "--remove"|"-r" => {
            let alias = matched_args.get_one::<String>("ALIAS").unwrap();
            cmd_remove(&alias, &mut cmd_groups);
        },
        "--completions" => {
            print!("print completions");
        },
        "--version" => {
            print!("{}", builder.render_version());
        },
        _ => {
            let args = match matched_args.get_many::<String>("args") {
                Some(s) => s.into_iter().map(|s| s.to_string()).collect(),
                None => vec![],
            };
            if let Some(command) = find_command(&(*subcommand).into(), &cmd_groups) {
                if command.scope.path.join(&command.rel_path).exists() {
                    let command_path = command.scope.path.join(&command.rel_path);
                    let command = command_path.into_os_string().into_string().expect("cannot convert path to string");
                    execute(&command, args);
                } else {
                    let alias = &command.alias;
                    let path_str = &command.rel_path;
                    println!("the {alias} alias is pointed to a non-existant file {path_str}");
                }
            } else {
                panic!("unknown subcommand returned from parser");
            }
        },
    }
}
