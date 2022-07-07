# Mtd - My Todo

Lightweight todo and task management app with built-in synchronization.

<!-- TODO: Add example gif/image -->

Mtd is a yet another todo app as enough of those don't exist yet. The main purpose of this app is to serve as simple
rust practice for me while also allowing me to sync my todos using my home server. However, compared to alternatives mtd
has some advantages, it's open source, lightweight and supports synchronization without additional programs.

I am also planning on writing an android app for mtd.

## Items

Mtd has two types of items: todos and tasks. Both items show have an id that can be used for marking them as done and
modifying them.

Todos are things that you expect to do once. As not all todos should be done immediately, it is possible to specify a
weekday for doing a todo. Done todos are automatically removed one day after completion.

Tasks are things that you expect to do repeatedly. When creating a new task, you should specify the weekdays for doing
the task.

## Installation

Currently, the only way to install mtd is manually using cargo.

```
cargo install --git https://github.com/Windore/mtd.git --features bin
```

Additionally, I am planning on publishing prebuilt binaries and an AUR package for mtd.

Mtd should be installed both locally and on the server. When running for the first time with a valid subcommand such
as `show`, mtd will create a config and prompt for some config options. This should be done both on the client and the
server.

```
> mtd show
Creating a new config.
Input server socket address (IP:PORT): 192.168.10.55:55995
Note! Encryption password is stored in cleartext but obfuscated locally.
Input encryption password: 
Input encryption password again: 
Initialize as a server or a client (s/c)? c
```

The encryption password should be the same on both the client(s) and the server. It is stored as an unencrypted
byte-array locally (example below). Encrypting it wouldn't provide basically any additional security since the saved
todos and tasks are stored unencrypted as well. The encryption password is only used for secure communication between a
client and the server. I'm telling you this to discourage you from reusing existing passwords.

If you want to run mtd only locally, input any valid socket address (it will be unused). Additionally, you'll have to
edit the mtd config manually after creating it. The config should reside in the user's config directory (for
Linux: `~/.config/mtd/conf.json`). Change `local_only` to `true`. Change `local_only` to `true`.

Example (this also shows the format in which the encryption password is stored. Use a longer password!):

```json
{
  "socket_addr": "127.0.0.1:1",
  "encryption_password": [
    115,
    101,
    99,
    117,
    114,
    101
  ],
  "timeout": {
    "secs": 30,
    "nanos": 0
  },
  "save_location": "/home/kk/.local/share/mtd/data.json",
  "local_only": true
}
```

## Examples

Mtd's command line help is pretty exhaustive but most important examples are still covered here.

Show help.

```
mtd --help
```

Show help about a subcommand

```
mtd <SUBCOMMAND> --help
```

Add a todo.

```
mtd add todo "Install mtd"
```

Add a todo for the next monday.

```
mtd add todo "Install mtd" mon
```

Add a todo for the next monday, tuesday and friday.

```
mtd add todo "Install mtd" mon tue fri
```

Add a task for each tuesday and friday.

```
mtd add task "Go grocery shopping" tue fri
```

Show todos and tasks for today.

```
mtd show
```

Show only todos for the next friday.

```
mtd show -i todo -w fri
```

Show todos and tasks for the next week.

```
mtd show --week
```

Set a todo as done

```
mtd do todo 0
```

Set a task as undone.

```
mtd undo task 1
```

Remove a task.

```
mtd remove task 4
```

Set a todo's text body and weekday to new values.

```
mtd set todo 3 -b "New text body" -w mon
```

Set a task's weekday's to mon tue and wed.

```
mtd set task 0 -w mon -w tue -w wed
```

Run a mtd server.

```
mtd server
```

Synchronize with a server.

```
mtd sync
```

## License

Copyright (C) 2022 Windore

Mtd is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as
published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

Mtd is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with this program. If not,
see <https://www.gnu.org/licenses/>.

See [license](LICENSE).

