# shell-scriptman

    Command-line tool for managing custom shell scripts

Licensed under [MIT](./LICENSE).

---

## Usage

```sh
git clone https://github.com/vaclavblazej/shell-scriptman.git
bash shell-scriptman/setup.sh
```

Default invocation of the command `cmd` yields:

```txt
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
* enable completion
