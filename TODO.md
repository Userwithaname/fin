# TODO:

[*] = Completed | [!] = Current priority | [-] = In progress | [ ] = Not started | [?] Uncertain

[*] Setup:
    [*] Load user arguments
        [*] Determine the action to perform (`install`/`remove`/`update`)
        [*] Determine which fonts to perform the action on
            [*] Error when no fonts were specified to `install` or `remove`
            [*] Error when one or more specified fonts aren't present in the list of all fonts
            [*] Use all installed fonts when running `update` with no specified fonts
        [*] Load additional arguments separately from the fonts (e.g.: `--test-argument`)
            [*] Handle the additional arguments so they can be used
        [*] Match fonts by wildcard pattern
        [*] Allow specifying a version override (for example: `font-name:v1.0`)
[*] Config:
    [*] Define a structure for keeping track of installed fonts and their relevant data
    [*] User configuration file (for example: for specifying the installation directory)
        IDEA: Allow multiple, configurable installer locations (used as fallbacks)
        IDEA: Allow fetching installers from remotely hosted locations (e.g. FTP repos)
[-] Metadata:
    [*] Use TOML files as installers
    [-] Find out the necessary information from the installer's specified URL
        [*] Download link
            Idea: It would be nice to have multiple 'modes', such as
                  `from-html`/`direct-link`/etc.
        [-] Actual version/tag
            For example, `latest` tag on GitHub could point to `v1.0`, in which case it
            could be useful to substitute it when parsing the installer (e.g. for composing
            links or archive names when required).
    [*] Remember locally installed fonts
    [*] Detect available updates
[*] Install fonts (`fin install [font]`):
    [*] Parse the installer
    [*] Validate arguments and tell the user which actions will be performed
    [*] Wait for user confirmation
    [*] Download the font into `~/.cache/fin/$font/$tag/`
    [*] Extract to a temporary location
    [*] Follow the instructions in the installer to install the font (e.g. into `~/.fonts`)
    [*] Add the font info entry to a file on disk
[*] Update fonts (`fin update` / `fin update [font]`):
    [*] Gather a list of installed fonts (or use only the fonts specified)
    [*] Exclude fonts with no available updates
    [*] Validate arguments and tell the user which actions will be performed
    [*] Wait for user confirmation
    [*] Perform the installation for each of the fonts
    [*] Update the font info entry in the file on disk
[-] Remove fonts (`fin remove [font]`):
    [*] Gather a list of installed fonts
    [-] Validate arguments and tell the user which actions will be performed
        TODO: Make sure the install path is valid (prevent potential data loss)
    [*] Wait for user confirmation
    [*] Delete from disk (install dir should be in the config)
    [*] Remove the font info entry from the file on disk

[ ] UX improvements:
    [ ] Show download size before downloading
    [ ] Show progress while downloading
    [ ] Download all archives prior to installing(?)
        [ ] Allow parallel downloads(?)
