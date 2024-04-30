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
  - Manage and Categorize Your Snippets
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
$ wclipd show
┌───┬─ default ─┬────┐
│ 0 │ hello     │ 6s │
│ 1 │ world!    │ 1s │
└───┴───────────┴────┘
```

Paste Older Copy Snippets using their Index

```bash
$ wclipd paste 0
hello
```

#### Configuration

Customize Wclipd Storage and Behavior using the available CLI flags
or via its [configuration file](./default-config.yaml).

#### Advanced Usage

Copy/Paste Images

```bash
$ cat <your-image.jpg> | wclipd copy
$ wclipd paste | feh -
```

Re-Copy an Old Entry to Active Clipboard

```bash
$ wclipd re-copy 0
```

Delete an Entry

```bash
$ wclipd delete 0
$ wclipd s
┌───┬─ default ─┬────┐
│ 1 │ world!    │ 5s │
└───┴───────────┴────┘
```

Categorize Your Entries into Groups On Input

```bash
$ wclipd copp ':)' --group smiles
$ wclipd s smiles
┌───┬─ smiles ─┬────┐
│ 0 │ :)       │ 3s │
└───┴──────────┴────┘
$ wclipd p -g smiles
:)
```

View Existing Groups

```bash
$ wclipd list-groups
┌─────────┬─────────┐
│ smiles  │ 57s     │
│ default │ 41s     │
└─────────┴─────────┘
```
