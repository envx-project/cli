# env-cli

## this CLI is in alpha and is not guaranteed to work

Author: [@alexng353](https://github.com/alexng353)

A very simple cli written for [env-store/envs](https://github.com/env-store/envs)

## Todo

- Revert to a single global configuration file

### Usage

```bash
Interact with env-store/envs via CLI

Usage: env-cli [OPTIONS] <COMMAND>

Commands:
  login      login to the service
  variables  Print all variables as either key=value pairs, json, or a table
  set        SET an environment variable with a key=value pair also supports interactive mode
  unset      UNSET an environment variable by key also supports interactive mode
  shell      Open a subshell with envcli variables available
  run        Run a local command using variables from the active environment
  help       Print this message or the help of the given subcommand(s)

Options:
      --json     Output in JSON format
  -h, --help     Print help
  -V, --version  Print version
```

## Attributions

This project is licensed under the GPLv3 License. A copy of the GPLv3 License can be found in the [LICENSE](LICENSE) file.

This project uses code from the [Railway's CLIv3](https://github.com/railwayapp/cli), copyright (c) [2023] Railway Corp. The Railway CLI is licensed under the MIT License. A copy of the MIT License can be found in the [attributions/railway/LICENSE](attributions/railway/LICENSE) file.
