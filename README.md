# env-cli

## this CLI is in alpha and is not guaranteed to work

Author: [@alexng353](https://github.com/alexng353)

A very simple cli written for [env-store/envs](https://github.com/env-store/envs)

## Todo

- Revert to a single global configuration file

## Installation

```bash
# MacOS, Linux
curl -fsSL get.envx.sh | bash
```

For windows users:
Download the binary from [this page](https://github.com/env-store/rusty-cli/releases/latest), then you can run that binary as an application.

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
  gen        Generate a key using GPG Saves the key to ~/.envcli/keys/<fingerprint>
  import     Import ascii armored keys from a file
  link       Get all environment variables for a project
  list-keys
  run        Run a local command using variables from the active environment
  set        Set a variable
  shell      Open a subshell with envcli variables available
  sign       Sign a message with a key
  unlink     Unset the current project
  unset      Unset (delete) an environment variable
  upload     If your key is not in the database, use this command to upload it
  variables  Get all environment variables for the current configured directory
  version    Fancy, pretty-printed version information
  config     Delete a resource. (project, key)
  delete     Delete a resource. (project, key)
  get        Get a resource. (project, key, config)
  keyring    Delete a resource. (project, key)
  new        Create a resource. (project)
  project    Command group for project related commands
  help       Print this message or the help of the given subcommand(s)

Options:
      --silent
  -h, --help     Print help
  -V, --version  Print version
```

## Attributions

This project is licensed under the MIT License. A copy of the MIT License can be found in the [LICENSE](LICENSE) file.

This project uses code from the [Railway's CLIv3](https://github.com/railwayapp/cli), copyright (c) [2023] Railway Corp. The Railway CLI is licensed under the MIT License. A copy of the MIT License can be found in the [attributions/railway/LICENSE](attributions/railway/LICENSE) file.
