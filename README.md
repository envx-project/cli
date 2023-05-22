# env-cli

Author: [@alexng353](https://github.com/alexng353)

A very simple cli written for [env-store/envs](https://github.com/env-store/envs)

## Todo

- find out how many bugs there are and fix em all

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

# License

```
MIT License

Copyright (c) 2022 alexng353

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

# Attributions

This project uses code from the [Railway's CLIv3](https://github.com/railwayapp/cli), copyright (c) [2023] Railway Corp. The Railway CLI is licensed under the MIT License. A copy of the MIT License can be found in the [attributions/railway/LICENSE](attributions/railway/LICENSE) file.
