# envx cli v2.0.1

## this CLI is in alpha and is not guaranteed to work

Author: [@alexng353](https://github.com/alexng353)

A very simple cli written for [envx-project/api](https://github.com/envx-project/api)

## Migration from v1 to v2

- envx config folder has been moved from `~/.config/envx` to `~/.config/envx/config.json`

Please run `envx config migrate` to migrate your config file to the new format.

<!-- ## Todo --> <!-- Who even wrote this? -->
<!---->
<!-- - Revert to a single global configuration file -->

## Installation

```bash
# MacOS, Linux
curl -fsSL get.envx.sh | bash
```

For windows users:
Download the binary from [this page](https://github.com/envx-project/cli/releases/latest), then you can run that binary as an application.

For more detailed instructions, see [windows installation](https://github.com/envx-project/cli/blob/main/windows-installation.md)

## setting a primary key password command

You can set a primary key password command (such as `op read "op://my secret
password"`) by setting `config.primary_key_command` in your config file.

```json
{
    ...
    "primary_key_command": ["op", "read", "op://my secret password"]
}
```

A command will be available for this in the future (maybe). See [#72](https://github.com/envx-project/cli/issues/72).

### Usage

```bash
Interact with envx servers via CLI

Usage: envx [OPTIONS] <COMMAND>

Commands:
  auth       Test authentication with the server
  export     Export a public or secret key
  gen        Generate a key using GPG Saves the key to ~/.config/envx/keys/<fingerprint>
  import     Import ascii armored keys from a file
  link       Link a project to the current directory
  list-keys  List all keys in the config
  run        Run a local command using variables from the active environment
  set        Set a variable (Interactive)
  shell      Open a subshell with envx variables available
  unlink     Unlink the current project
  unset      Unset (delete) an environment variable
  update     Attempt to self-update envx using the installation script. Fails on Windows
  upload     If your key is not in the database, use this command to upload it
  variables  Get all environment variables for the current configured directory
  version    Fancy, pretty-printed version information
  whoami     Print the primary key fingerprint and uuid
  config     Configure envx
  delete     Delete a resource. (project, key)
  get        Get a resource. (project, key, config)
  keyring    Interact with the envx keyring. All commands are interactive
  project    Command group for project related commands
  help       Print this message or the help of the given subcommand(s)

Options:
      --silent
  -h, --help     Print help
  -V, --version  Print version
```

## Attributions

This project is licensed under the GPLv3 License. A copy of the GPLv3 License can be found in the [LICENSE](LICENSE) file.

This project uses code from the [Railway's CLIv3](https://github.com/railwayapp/cli), copyright (c) [2023] Railway Corp. The Railway CLI is licensed under the MIT License. A copy of the MIT License can be found in the [attributions/railway/LICENSE](attributions/railway/LICENSE) file.
