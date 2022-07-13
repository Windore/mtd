# Mtd - My Todo

![example workflow](https://github.com/Windore/mtd/actions/workflows/rust.yml/badge.svg)

Lightweight todo and task management app with built-in encrypted synchronization support, written in Rust.

https://user-images.githubusercontent.com/65563192/178841983-8f8c5aec-7b46-42bd-9484-fbf4aa73ec5d.mp4

Mtd is a yet another todo app as enough of those don't exist yet. However, mtd has some benefits over the existing apps,
mtd has built-in synchronization, it's lightweight and has a clean CLI. I am also planning on writing an Android app for
mtd. As for security, all network transmissions for synchronization are encrypted using AES GCM.

<details>
  <summary>Mtd's synchronization follows a star topology</summary>
  
  Mtd's synchronization works by having a single machine function as a server. Other devices then connect to that server.
  Having an external server machine is helpful, but not necessary, as a mtd server can be run on a desktop machine
  alongside the normal client instance. The server is packed into the same binary so installing anything extra is not
  required.
  
</details>

<details>
  <summary>Mtd supports one-time todos and repeating tasks.</summary>
  
  Both items have an id that can be used for marking them as done and modifying them. 
  
  Todos are things that you expect to do once. As not all todos should be done immediately, it is possible to specify
  a weekday for doing a todo. Done todos are automatically removed one day after completion. 
  
  Tasks are things that you expect to do weekly. When creating a new task, you should specify the weekdays for doing the task.
  
</details>



**Note, mtd is not a calendar.** It only supports todos and tasks both of which are dated using only weekdays.

## Installation

Currently, the only way to install mtd is manually using cargo.

```
cargo install --git https://github.com/Windore/mtd.git --features bin
```

I am planning on publishing prebuilt binaries and an AUR package for mtd.

## Using mtd

Mtd should be installed both locally and on the server. When running for the first time with a valid subcommand such
as `show`, mtd will create a config and prompt for some config options. This should be done both on the client and the
server.

```
> mtd show
Creating a new config.
Create a local only instance (y/n)? n
Input server socket address (IP:PORT): 127.0.0.1:55995
Note! Encryption password is stored in cleartext but obfuscated locally.
Input encryption password:
Input encryption password again:
Input save path (Leave empty for default):
Initialize as a server or a client (s/c)? c
```

When initializing a server, it is recommended to input 127.0.0.1 as the server's IP address.

The encryption password should be the same on both the client(s) and the server. It is stored as an unencrypted
byte-array locally. Encrypting it wouldn't provide basically any additional security since the saved
todos and tasks are stored unencrypted as well. The encryption password is only used for secure communication between a
client and the server.

### Running a server and a client on the same machine

When running a server on a same machine as a client, the server needs to have a separate config and a data file. This is
accomplished with the `--config-file` option and a different save path.

```
> mtd --config-file server/only/config/file server
...
Input save path (Leave empty for default): server/only/data/file
...
```

### Examples

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

Run a mtd server using a different config file.

```
mtd --config-file /path/to/server/config server
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

