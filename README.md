# localdeploy

localdeploy downloads new commits from a given git repository and runs a specified command to build or run the project.

> Note: cloning (with `--new`) over https is currently not impemented.
> You'll get a Error: ` GitError(Error { code: -16, klass: 34, message: "remote authentication required but no callback set" })`

## Usage

```
localdeploy 1.0
Ceeox <mizuo@pm.me>

USAGE:
    localdeploy [FLAGS] [OPTIONS]

FLAGS:
    -h, --help              Prints help information
    -s, --use-passphrase    Give a hint if the ssh private is protected by a passphrase
    -V, --version           Prints version information

OPTIONS:
    -b, --branch <BRANCH>              Provides a default branch to fetch repo from [default: main]
    -c, --command <CMD>                Command to run the project
    -i, --interval <INTERVAL>          Interval between each git fetch in sec [default: 3600]
    -n, --new <REPO_URL>               Url to the new git repo. Ensure a path to where the repo should to cloned to.
    -p, --path <PATH>                  File path to the existing repo
        --private-key <PRIVATE_KEY>    Path to the private ssl key [default: ~/.ssh/id_rsa]
        --public-key <PUBLIC_KEY>      Path to the public ssl key [default: ~/.ssh/id_rsa.pub]
    -r, --remote <REMOTE>              Provides a default origin to fetch repo from [default: origin]
    -u, --username <USERNAME>          Username for git auth [default: git]
```

## Examples

- cloning a project:
    ```
    localdeploy --new git@github.com:<YOU>/<YOUR_PROJECT>.git --command "cargo build" --path ../<YOUR_PROJECT>
    ```

- Normal usage with a already cloned repository:
    ```
    localdeploy --path ./<YOUR_PROJECT> --command "cargo run --release"
    ```

- With a passphrase protected ssh key:
    ```
    localdeploy --path ./<YOUR_PROJECT> --command "cargo run --release" --use-passphrase
    ```
