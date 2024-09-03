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

### Usage

```bash
Interact with envx servers via CLI

Usage: envx [OPTIONS] <COMMAND>

Commands:
  auth       Test authentication with the server
  debug      Unset the current project
  decrypt    Decrypt a string using GPG
  encrypt    Encrypt a string using GPG
  export     Export a public or secret key
  gen        Generate a key using GPG Saves the key to ~/.envx/keys/<fingerprint>
  import     Import ascii armored keys from a file
  link       Get all environment variables for a project
  list-keys  List all keys in the config
  run        Run a local command using variables from the active environment
  set        Set a variable (Interactive)
  shell      Open a subshell with envx variables available
  sign       Sign a message with a key
  unlink     Unset the current project
  unset      Unset (delete) an environment variable
  upload     If your key is not in the database, use this command to upload it
  variables  Get all environment variables for the current configured directory
  version    Fancy, pretty-printed version information
  config     Configure envx
  delete     Delete a resource. (project, key)
  get        Get a resource. (project, key, config)
  keyring    Interact with the envx keyring. All commands are interactive
  new        Create a resource. (project)
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
