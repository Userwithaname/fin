> [!CAUTION]
> This software is still in active development, and may or may not maintain
> backward/forward compatibility. Some configurations may be untested.

> [!NOTE]
> Fin is a learning project, and my first actual project written in Rust.
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

- `name`
    > The name of the font, used as the installation directory
- `source`
    > Where to obtain the font from
    - `[source.GitHub]`
        > Download releases of a GitHub project
        - `tag`
            > Tag/version of the font to install
            > (optional, defaults to "latest")
        - `author`
            > GitHub project author
        - `project`
            > GitHub project name
    - `[source.Webpage]`
        > Download from a webpage
        - `tag`
            > Arbitrary value (optional unless other fields use `$tag`)
        - `url`
            > Download page URL, which must contain a download link to
            > `file` within its source
    - `[source.Direct]`
        > Specify a direct link to `file`
        - `tag`
            > Arbitrary value (optional unless other fields use `$tag`)
        - `url`
            > Note: The URL must end with `$file`
        > Note: Direct links cannot currently detect updates except by
        > manually changing the `url`
- `action`
    > Specify what to do with the file
    - `[action.Extract]`
        > Use to extract files from the `$file` archive
        - `file`
            > Name of the file to download and extract from
            > (supports wildcards, except for direct links)
        - `include`
            > Specify which files within the archive to install
        - `exclude`
            > Specify which files to ignore
            > (optional: takes precedence over `include`, defaults to none)
        - `keep_folders`
            > Follow the same directory structure as the archive
            > (optional, defaults to `false`)
        > Note: The `include` and `exclude` fields support wildcards
    - `[action.SingleFile]`
        > Use to install the downloaded file with no processing action
        - `file`
            > Name of the file to download
            > (supports wildcards)
- `check`
    > Optionally specify an integrity check method
    - `[check.SHA256]`
        > Specify the file on the webpage containing the checksum string, or
        > leave unspecified to look for the checksum within the page contents
        - `file`
            > The checksum file to download (optional)
    > Note: Not supported for direct download links

## Example installers

A file in `~/.config/fin/installers/` named `maple-mono` with the following
contents would enable Fin to install the latest release of
[Maple Mono](https://github.com/subframe7536/maple-font) directly from GitHub
by running `fin install maple-mono`, and keep it updated using `fin update`:

```toml
name = "Maple Mono"

[source.GitHub]
tag = "latest"
author = "subframe7536"
project = "maple-font"

[action.Extract]
file = "MapleMono-Variable.zip"
include = [ "LICENSE.txt", "*.ttf" ]

[check.SHA256]
file = "MapleMono-Variable.sha256"
```

Fin is able to install from other sources as well. For example, GeistMono
Nerd Font from the [Nerd Fonts](https://www.nerdfonts.com/) website:

```toml
name = "GeistMono Nerd Font"

[source.Webpage]
url = "https://www.nerdfonts.com/font-downloads"

[action.Extract]
file = "GeistMono.zip"
include = [ "*" ]
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
