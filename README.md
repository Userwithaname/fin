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

Fin is a font manager which allows you to install and manage fonts using
custom installers. Installers tell Fin how each font should be obtained
and how to install it. This allows fonts to be installed directly from
the source (such as release assets of GitHub repositories).
Fonts can be managed using the `install`/`update`/`remove` commands, akin
to a standard package manager.

"Fin" is a contraction of "font installer".

# Usage

- `fin install [fonts]` - installs the specified fonts
- `fin update [fonts (optional)]` - updates your installed fonts
- `fin remove [fonts]` - removes the specified fonts
- `fin help [action]` - help messages for each action

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
    list                  List installed or available fonts
    clean                 Remove temporary cache files
    config                Manage the configuration file
    version               Show the current version number
    help                  Show help for any action

Arguments:
    --refresh       -r    Ignore cache and fetch new data
    --no-refresh    -c    Do not fetch new data if possible
    --reinstall     -i    Skip version checks and reinstall
    --verbose       -v    Show more detailed output
    --force         -F    Forcefully perform action (unsafe)
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
    - `[action.Extract]` - extract files from the archive
        - `include` - specify which files within the archive to install
        - `exclude` - specify which files to ignore (takes precedence over `include`, defaults to none)
        - `keep_folders` - follow the same directory structure as the archive (defaults to `false`)
    - `[action.SingleFile]` - place the downloaded file into the installation directory directly
- `check` - optionally specify an integrity check method
    - `[check.SHA256]`
        - `file` - the file containing the checksum string

> [!IMPORTANT]
> Unless using direct links, Fin looks for the font's download
> link within the webpage source. If the site layout, links, or
> files change, the installer may need to be updated as well.
> Note that in order for installers to work, the download link
> must be accessible without JavaScript.

> [!NOTE]
> Installers using direct links currently cannot detect updates
> without changing the installer.

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

Fin can be configured using the `config.toml` file located in
`~/.config/fin/`.

Running `fin config write-default` will create the following default
configuration:

```toml
# Location where new fonts will be installed
# Override:  --install-dir=[path]
install_dir = "~/.local/share/fonts"

# How long (in minutes) until cache is considered outdated
# Override:  --cache-timeout=[time]
# Related:   --refresh, --no-refresh
cache_timeout = 90

# Show verbose output by default
# Enable:   --verbose
# Disable:  --no-verbose
verbose_mode = false

# Show verbose output when adding or removing files
# Enable:   --verbose-files,    --verbose
# Disable:  --no-verbose-files, --no-verbose
verbose_files = false

# Show installed paths when running the list command
# Enable:   --verbose-list,    --verbose
# Disable:  --no-verbose-list, --no-verbose
verbose_list = false

# Show URLs in the output
# Enable:   --verbose-urls,    --verbose
# Disable:  --no-verbose-urls, --no-verbose
verbose_urls = false
```

# Building from source

1. [Install Rust and Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
2. Clone this repository: `git clone https://github.com/Userwithaname/fin.git`
3. Enter the cloned directory: `cd fin`
4. Build it using `cargo build` — the program binary will appear in `…/target/debug/fin`
5. To run it, either:
    - Put the program binary into a location within your `$PATH` to run it
    using the `fin` command, or
    - Run `./target/debug/fin` from the `fin` directory, or
    - Run it using Cargo: `cargo run -- [action] [items]`

> [!NOTE]
> Building may require `openssl-devel` to be installed on your system.

To learn more, see the Cargo documentation for
[`cargo build`](https://doc.rust-lang.org/cargo/commands/cargo-build.html)
and [`cargo run`](https://doc.rust-lang.org/cargo/commands/cargo-run.html).
