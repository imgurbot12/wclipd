## wclipd

A Customizable Clipboard Manager for Wayland

<!--
  TODO: include dmenu/rmenu support
  TODO: rewrite wayland underlying clipboard stack
-->

### Features
  - Blazingly Fast 🔥
  - Simple and Easy to Use
  - Unifies Clipboard Tooling into One Binary
  - Multiple and Customizable Storage Options
  - Exact Control on How Long your Copies last

### Why

After looking at existing clipboard management tools for wayland,
**NONE of the existing solutions have all the features I wanted in one
easy-to-use tool.**

Second, **Having an all-in-one daemon avoids many of the weird
hacks required by other solutions due to the nature of wayland
and its protocols.**

Due to waylands design, in order for a copy snippet to remain
available, a process that includes that snippet must always be running.
Tools like `wl-clipboard` use dirty hacks to spawn a fork of themselves
to sit and wait in the background so you can paste snippets copied from
the terminal. _Using a unified daemon avoids these problems._

### Installation

```bash
$ make install
```

### Usage

View all available options and commands via the built-in help

```bash
$ wclipd --help
```

Ensure the Daemon is Running in the Background.
Easy to Include in Your Sway Config For Example.

```bash
$ wclipd daemon
```

Copy and Paste via Terminal with Ease


```bash
$ wclipd copy 'hello world!'
$ wclipd paste
```

View a History of Available Snippets. Previews are listed
from most-recent copy to least.

```bash
$ wclipd copy 'hello'
$ wclipd copy 'world!'
$ wclipd list
2.      world!
1.      hello
```

Paste Older Copy Snippets using their Index

```bash
$ wclipd paste 1
hello
```

#### Configuration

Customize Wclipd Storage and Behavior using the available CLI flags
or via its [configuration file](./default-config.yaml).

#### Examples

Copy/Paste Images

```bash
$ cat <your-image.jpg> | wclipd copy
$ wclipd paste | feh -
```

Restart/Reactivate Daemon (and killing the old one)

```bash
$ wclipd daemon -k -b memory -l login
```

