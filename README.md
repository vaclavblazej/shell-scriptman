# shell-scriptman

This program allows users to efficiently manage and execute custom shell scripts by organizing them into local or global scopes, creating new scripts, editing existing ones, or removing obsolete entries—all with a simple interface.

Licensed under [MIT](./LICENSE).

## Description

This tool simplifies the management of custom scripts across global and project-specific scopes.
By using `cmd <name>`, you can invoke a global script from any directory.
Commands for script management begin with a dash or double dash.

You can create a project scope with `cmd --init`, enabling the management of scripts that are active only when the current directory is within the project directory.

All scripts are stored in a hidden `.cmd` folder.
For project scopes, this folder is located in the project root where `--init` was run.
For the global scope, the folder is within the `cmd` command's installation directory, which you can usually find by running `whereis cmd`.

Script is not invoked through a specific shell, it is run directly.
To setup shell used for its invocation use shebang on its first line, for example:

```sh
#!/usr/bin/env bash
#!/usr/bin/env zsh
#!/usr/bin/env fish
```

We avoid setting up any extra variables by invoking the script from the current working directory.
To make a script work from the project root add the following code to the beginning of the script.

```sh
cd "$(dirname "$0")/../.." || exit
```

## Installation

Requries `cargo` to compile the rust binary.

```sh
git clone https://github.com/vaclavblazej/shell-scriptman.git
bash shell-scriptman/setup.sh
```

## Usage

Running command `cmd` results in:

```txt
$ cmd
Usage: cmd [OPTIONS] <COMMAND>

Commands:
  --init     Setup local scope in the current directory [aliases: -i]
  --add      Create script and open it in the $EDITOR [aliases: -a]
  --edit     Open script index or [SCRIPT] in the $EDITOR [aliases: -e]
  --remove   Remove script from the index (does NOT remove file) [aliases: -r]
  --version  Prints out version information

Options:
  -l, --local   Force local scope
  -g, --global  Force global scope
```

The command holds custom scripts in a hidden folder.
To create custom commands in the current folder, run:

```sh
cmd --init
cmd --add hello "Test script that prints 'Hello world!'"
```

These commands open your `$EDITOR` to edit the hello script.
Save it and observe that the following structure was created:

```txt
.
└── .cmd
    ├── scripts
    │   └── hello.sh
    └── index.json
```

Invoke `cmd` help to see your `hello` script added and you may now run it which prints `Hello world!`.
Note that this works from any subfolder of the folder where you initialized the local scope.

```sh
cmd hello
```

Edit the script or the index of all your commands with `--edit` command.

```sh
cmd --edit hello
cmd --edit
```

Finally, you may remove a script via `--remove` command.

```sh
cmd --edit hello
```

## Scopes

The examples above show how to add commands to local scope -- an initialized directory.
One may also add commands to *gobal* scope by running the command without an initialized directory.
An operation may be forced to work in some scope with options `--global` and `--local`.
For example, if you want to edit global scope even though you are inside a directory with local scope, invoke:

```sh
cmd --global --edit
```

## Todos

* release to `crates.io`
* split help print of global, local, and management commands
