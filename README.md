# gt - Gitea CLI
Copyright (c) 2024 Paul Sobolik

This could be a command line interface to Gitea like the GitHub API someday,
but for now it only has three subcommands, `repo create`, `repo browse` and `repo search`.

The `create`  command also tells Git to track the remote locally, which may not 
be a good choice for a generic tool, but which is pretty much the main thing I 
want this tool to do.

## `gt repo create --help`
```
Create a new Gitea repository and track it locally

Usage: gt.exe repo create [OPTIONS] --gitea-url <url>

Options:
      --path <path>                Local path [default: current folder]
      --gitea-url <url>            Gitea server URL
  -d, --description <description>  Description
      --gitea-name <gitea_name>    Gitea repository name [default: top-level Git folder]
  -b, --branch <default_branch>    Default branch [default: main]
      --remote <remote>            Remote name [default: origin]
      --private                    Make repository private
      --template                   Make repository a template
      --trust-model <trust_model>  Trust model; Default, Collaborator, Committer, or CollaboratorCommitter [default: Default]
  -h, --help                       Print help
```

## `gt repo browse --help`
```
Open the remote repository in a browser

Usage: gt.exe repo browse [OPTIONS]

Options:
      --remote <remote>  Remote name [default: origin]
      --path <path>      Local path [default: current folder]
  -h, --help             Print help
```
## `gt repo search --help`
```
Search remote repositories

Usage: gt.exe repo search [OPTIONS] --gitea-url <url>

Options:
      --gitea-url <url>      Gitea server URL
      --contains <contains>  Only find remotes whose name contains this value
  -h, --help                 Print help
```
