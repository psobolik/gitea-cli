# gt - Gitea CLI
Copyright (c) 2024 Paul Sobolik

This could be a command line interface to Gitea like the GitHub API someday,
but for now it only has two subcommands, `repo create` and `repo browse`.

## `gt repo create`
```
Create a new Gitea repository and track it locally.

Usage: gt.exe repo create [OPTIONS] --gitea-url <url>

Options:
--path <path>                       Local path [default: current folder]
--gitea-url <url>                   Gitea server URL
-d, --description <description>     Description
--gitea_name <gitea_name>           Gitea repository name [default: top-level Git folder]
-b, --branch <default_branch>       Default branch [default: main]
--remote <remote>                   Remote name [default: origin]
--private                           Make repository private
--template                          Make repository a template
--trust-model <trust_model>         Trust model; Default, Collaborator, Committer, or CollaboratorCommitter [default: Default]
--issue-labels <issue_labels>       Issue labels
-h, --help                          Print help
```

## `gt repo browse`
```
Open the remote repository in a browser

Usage: gt.exe repo browse [OPTIONS]

Options:
--remote <remote>  Remote name [default: origin]
--path <path>      Local path [default: current folder]
-h, --help         Print help
```
