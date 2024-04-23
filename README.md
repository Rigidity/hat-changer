# Hat Changer

![Crates.io Version](https://img.shields.io/crates/v/hat-changer)


Hat Changer is a very simple command-line tool written in Rust for tracking time for various projects.

## Installation

First, you will need to install Rust with [Rustup](https://rustup.rs/).

Then run the following command:

```bash
cargo install hat-changer
```

## Usage

For more detailed instructions, see:

```bash
hat --help
```

You can create projects with `new`:

```bash
hat new project-name
hat new another-project
```

Change hats by typing the name of the project:

```bash
hat project-name
hat another-project
```

And start tracking time with the following commands:

```bash
hat on
hat off Description of what you've done.
```

You can undo or edit how long a task took. Note that if you undo while tracking time, it will just cancel the current time being tracked.

Here is an example:

```bash
hat edit 5h
hat undo
```

You can see a list of projects and the times for the active project with:

```bash
hat list
hat time
hat
```

Finally, if you ever want to, you can delete a project:

```bash
hat delete project-name
hat delete another-project
```

That's all for now! I may add new functionality for manipulating descriptions and historical tasks in the future, as well as archiving. But for now, you can do anything else you need by editing the `~/.timelogger.json` file directly. Contributions are welcome.
