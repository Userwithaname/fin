> [!CAUTION]
> This software is still in early development. Features could be missing,
> incomplete, or fully or partially broken. Misconfigurations or bugs in
> the code could potentially result in data loss. Use it at your own risk.

> [!NOTE]
> Fin is a learning project, and my first "real" project written in Rust.
> As such, the code quality may not be up-to-par with your expectations.
> Feel free to try it in a VM, report the issues you encounter, share
> ideas, suggest improvements, etc, etc. Enjoy and have fun! 😄

# About Fin

Fin is a font installer and manager for your terminal, which lets you create
your own installers. Installers tell Fin how and where to obtain each font
directly from its source (such as GitHub releases) and manage them on your
system. Fonts can be managed using the `install`/`update`/`remove` commands,
akin to a standard package manager.

"Fin" is a contraction of "font installer".

# Usage

- `fin install` - installs the specified fonts
- `fin update` - updates your installed fonts
- `fin remove` - removes the specified fonts

See the output of `fin help` for more information:

```
$ fin help
Usage:
    fin [action] [items]

Actions:
    install               Install new fonts
    reinstall             Reinstall fonts
    update                Update installed fonts
    remove                Remove installed fonts
    clean                 Remove temporary cache files
    list                  List installed or available fonts
    init                  Create a new configuration file
    help                  Show this help message

Arguments:
    --install-dir=[path]  Set the installation directory
    --reinstall     -i    Skip version checks and reinstall
    --refresh       -r    Ignore cache and fetch new data
    --cache-only    -c    Do not fetch new data if possible
    --verbose       -v    Show more detailed output
    --force         -f    Forcefully perform action (unsafe)
    --yes           -y    Automatically accept prompts
    --no            -n    Automatically reject prompts
```

Note that you will also need an installer for any font you wish to install.

# Installers

Fin relies on TOML files (located in `~/.config/fin/installers/`) to specify
how each font should be installed. You can find a few examples in the
`installers/` directory of this repository.

Using the installers, Fin will attempt to locate and download the font archive,
and install the font on your system.

Supported fields are as follows:

- `name` - name of the font, used for the installation directory
- `tag` - the tag/version of the font to install
- `url` - the URL to the font's download page (append `$file` for direct links)
- `file` - the name of the file to download
- `action` - what to do with the file:
    - `[action.Extract]` - extract files from the archive; required fields:
        - `include` - specify which files within the archive to install
        - `exclude` - specify which files to ignore (takes precedence over `include`, defaults to none)
        - `keep_folders` - follow the same directory structure as the archive (defaults to `false`)
    - `[action.SingleFile]` - place the downloaded file into the installation directory directly

> [!IMPORTANT]
> Unless using direct links, the font download is found within a plain
> text version of the webpage. If the site layout or font assets change,
> the installer may need to be updated as well. Updating fonts using
> direct links is currently not supported.

> [!NOTE]
> You can use `$name` or `$tag` as placeholders for their values
> in all fields except `name` or `tag`.

> [!NOTE]
> The `file`/`include`/`exclude` fields support wildcards.
> Wildcards are not supported inside direct link URLs.

## Example installer

Creating a `maple-mono` file in `~/.config/fin/installers/` with the
following contents will allow you to install the latest version of
[Maple Mono](https://github.com/subframe7536/maple-font) from GitHub
by running `fin install maple-mono`:

```toml
name = "Maple Mono"
tag = "latest"
url = "https://api.github.com/repos/subframe7536/maple-font/releases/$tag"
file = "MapleMono-Variable.zip"

[action.Extract]
include = [ "LICENSE.txt", "*.ttf" ]
````

# Configuration

Fin can be configured using the `config.toml` file in your `~/.config/fin/`
directory.

Running `fin init` will create the following configuration:

```toml
# Default location for installing new fonts
install_dir = "~/.local/share/fonts"

# How long (in minutes) until cache is considered outdated
cache_timeout = 90

# Show verbose output by default (pass --no-verbose to negate)
verbose_mode = false

# Show installed paths when running the 'list' command
verbose_list = false

# Show verbose cache-related output
verbose_cache = false
```

# Building from source

If you wish to use Fin, you must first build it from source:

1. [Install Rust and Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
2. Clone this repository: `git clone https://github.com/Userwithaname/fin.git`
3. Enter the cloned directory: `cd fin`
4. Build it using `cargo build` — the program binary will appear in `…/target/debug/fin`
5. To run it, either:
    - Run it using Cargo: `cargo run -- [action] [items]`, or
    - Run `./target/debug/fin` from the `fin` directory, or
    - Put the program binary into a location within your `$PATH` (such as `~/.local/bin/`)
    so you can run it from anywhere using the `fin` command

To learn more, see the Cargo documentation for
[`cargo build`](https://doc.rust-lang.org/cargo/commands/cargo-build.html)
and [`cargo run`](https://doc.rust-lang.org/cargo/commands/cargo-run.html).
