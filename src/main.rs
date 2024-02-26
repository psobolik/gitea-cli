mod error;
mod macros;

pub extern crate colored;

use crate::error::CliError;
use clap::{command, Arg, ArgAction, ArgMatches, Command};
use git_lib::{Credentials, GitLib};
use gitea_api::{CreateRepoOptions, GiteaApi, Repository, SearchReposResult, TrustModel};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

#[tokio::main]
async fn main() {
    let matches = get_clap_builder_command().get_matches();

    match matches.subcommand() {
        Some(("repo", repo_sub_matches)) => match repo_sub_matches.subcommand() {
            Some(("search", search_sub_matches)) => {
                search_command(search_sub_matches).await;
            }
            Some(("browse", browse_sub_matches)) => {
                browse_command(browse_sub_matches);
            }
            Some(("create", create_sub_matches)) => {
                create_command(create_sub_matches).await;
            }
            _ => unreachable!("Unexpected repo subcommand"),
        },
        _ => unreachable!("Unexpected subcommand"),
    }
}

async fn create_command(matches: &ArgMatches) {
    // Get the required Gitea server URL.
    let url = match matches.get_one::<String>("url") {
        Some(url) => url,
        _ => {
            print_error!("Missing Gitea URL");
            return;
        }
    };
    // Get the optional top-level path, and make sure it is inside a repository.
    // (If there's no path, this uses the current folder.)
    let path: Option<PathBuf> = matches.get_one::<String>("path").map(PathBuf::from);
    let top_level = match GitLib::top_level(path.as_ref()) {
        Ok(top_level) => top_level,
        Err(error) => {
            print_error!("{}", error);
            return;
        }
    };
    // Get the credentials for the Gitea server
    let credentials = match GitLib::credentials_fill(url) {
        Ok(credentials) => credentials,
        Err(error) => {
            print_error!("{}", error);
            return;
        }
    };
    // Get the required remote name.
    let remote = match matches.get_one::<String>("remote") {
        Some(remote) => remote,
        _ => {
            print_error!("Missing remote name");
            return;
        }
    };
    // Get the optional path.
    let path: Option<PathBuf> = matches.get_one::<String>("path").map(PathBuf::from);

    // If the Gitea repository name hasn't been specified, calculate it from the path
    let gitea_name = match matches.get_one::<String>("gitea_name") {
        Some(gitea_name) => gitea_name.to_string(),
        _ => suggested_remote_repo_name(&top_level),
    };
    // Get options in a struct that Gitea needs to create a repository.
    let options = repo_options(gitea_name.as_str(), matches);

    match create_repository(url, &credentials, &options).await {
        Ok(repository) => {
            print_info!("Created remote repository: {}", repository.clone_url);
            if let Err(error) =
                GitLib::remote_add(remote, repository.clone_url.as_str(), path.as_ref())
            {
                print_error!("{}", error.to_string())
            } else {
                print_info!("Tracking remote repository locally as: {}", remote);
                print_info!("Push: git push -u {} {}", remote, repository.default_branch);
            }
        }
        Err(error) => print_error!("{}", error),
    }
}

async fn search_command(matches: &ArgMatches) {
    // Get the required Gitea server URL.
    let url = match matches.get_one::<String>("url") {
        Some(url) => url,
        None => {
            print_error!("Missing Gitea URL");
            return;
        }
    };
    // Get the optional filter
    let contains = matches.get_one::<String>("contains");
    match contains {
        Some(contains) => print_info!("Repositories on '{}' containing '{}':", url, contains),
        None => print_info!("Repositories on '{}'", url),
    }

    match search_repos(url, contains).await {
        Ok(result) => {
            if result.ok() {
                if result.repositories().is_empty() {
                    println!("<no matches>");
                } else {
                    println!("Full Name | Clone URL | Description");
                    for repo in result.repositories() {
                        println!(
                            "{} | {} | {}",
                            repo.full_name,
                            repo.clone_url,
                            if repo.description.is_empty() {
                                "<no description>"
                            } else {
                                repo.description.as_str()
                            }
                        );
                    }
                }
            }
        }
        Err(error) => print_error!("{}", error),
    }
}

fn browse_command(matches: &ArgMatches) {
    // Get the required remote name.
    let name = match matches.get_one::<String>("remote") {
        Some(name) => name,
        None => {
            print_error!("Missing remote name");
            return;
        }
    };
    // Get the optional path.
    let path: Option<PathBuf> = matches.get_one::<String>("path").map(PathBuf::from);
    open_git_remote(name.as_str(), path.as_ref())
}

fn get_clap_builder_command() -> Command {
    let remote_arg = Arg::new("remote")
        .help("Remote name")
        .long("remote")
        .default_value("origin");

    let path_arg = Arg::new("path")
        .help("Local path [default: current folder]")
        .long("path")
        .required(false);

    // WTF?
    let gitea_url = Box::new(std::env::var("GITEA_URL").unwrap_or_else(|_| String::new())).leak();
    let mut gitea_url_arg = Arg::new("url")
        .help("Gitea server URL")
        .long("gitea-url")
        .required(gitea_url.is_empty());
    if !gitea_url.is_empty() {
        gitea_url_arg = gitea_url_arg.default_value(OsStr::new(gitea_url));
    }

    let gitea_name_arg = Arg::new("gitea_name")
        .help("Gitea repository name [default: top-level Git folder]")
        .long("gitea-name");

    let default_branch_arg = Arg::new("default_branch")
        .help("Default branch")
        .short('b')
        .long("branch")
        .default_value("main");

    let description_arg = Arg::new("description")
        .help("Description")
        .short('d')
        .long("description");

    let private_arg = Arg::new("private")
        .help("Make repository private")
        .long("private")
        .action(ArgAction::SetTrue);

    let template_arg = Arg::new("template")
        .help("Make repository a template")
        .long("template")
        .action(ArgAction::SetTrue);

    let trust_model_arg = Arg::new("trust_model")
        .help("Trust model; Default, Collaborator, Committer, or CollaboratorCommitter")
        .long("trust-model")
        .default_value("Default");

    let contains_arg = Arg::new("contains")
        .help("Only find remotes whose name contains this value")
        .long("contains")
        .required(false);

    command!().arg_required_else_help(true).subcommand(
        Command::new("repo")
            .about("Work with Gitea repositories")
            .arg_required_else_help(true)
            .subcommand(
                Command::new("search")
                    .about("Search remote repositories")
                    .arg(&gitea_url_arg)
                    .arg(&contains_arg),
            )
            .subcommand(
                Command::new("browse")
                    .about("Open the remote repository in a browser")
                    .arg(&remote_arg)
                    .arg(&path_arg),
            )
            .subcommand(
                Command::new("create")
                    .about("Create a new Gitea repository and track it locally")
                    .arg(path_arg)
                    .arg(gitea_url_arg)
                    .arg(description_arg)
                    .arg(gitea_name_arg)
                    .arg(default_branch_arg)
                    .arg(remote_arg)
                    .arg(private_arg)
                    .arg(template_arg)
                    .arg(trust_model_arg),
            ),
    )
}

async fn create_repository(
    url: &str,
    credentials: &Credentials,
    options: &CreateRepoOptions,
) -> Result<Repository, CliError> {
    let gitea_api = GiteaApi::new(
        url,
        credentials.username().as_deref(),
        credentials.password().as_deref(),
    );
    match gitea_api.create_repo(options).await {
        Ok(repository) => Ok(repository),
        Err(error) => Err(error::CliError::from(error)),
    }
}

async fn search_repos(url: &str, contains: Option<&String>) -> Result<SearchReposResult, CliError> {
    // Doesn't need credentials
    let gitea_api = GiteaApi::new(url, None, None);
    match gitea_api.search_repos(contains).await {
        Ok(repository) => Ok(repository),
        Err(error) => Err(error::CliError::from(error)),
    }
}

fn open_git_remote(repo: &str, path: Option<&PathBuf>) {
    match GitLib::remote_url(repo, path) {
        Ok(remote_url) => {
            let ru = <String as AsRef<OsStr>>::as_ref(&remote_url);
            match open::that_detached(ru) {
                Ok(()) => print_info!("Opened '{}'", remote_url),
                Err(error) => print_error!("Error opening '{}': {}", remote_url, error),
            }
        }
        Err(error) => print_error!("Error getting remote URL for '{}': {}", repo, error),
    }
}

fn repo_options(name: &str, matches: &ArgMatches) -> CreateRepoOptions {
    let description = matches.get_one::<String>("description");
    let default_branch = matches
        .get_one::<String>("default_branch")
        .expect("Missing default branch");
    let private = matches.get_flag("private");
    let template = matches.get_flag("template");

    let trust_model = if let Some(trust_model) = matches
        .get_one::<String>("trust_model")
        .map(|s| s.parse().unwrap())
    {
        trust_model
    } else {
        TrustModel::Default
    };
    /*
        println!("Name:           {}", name);
        println!("Description:    {:?}", description.map(|x| x.to_string()));
        println!("Default branch: {}", default_branch);
        println!("Private:        {}", private);
        println!("Template:       {}", template);
        println!("Trust model:    {}", trust_model);
    */
    CreateRepoOptions::new(
        name.to_string(),                   // name: String,
        default_branch.to_string(),         // default_branch: String,
        trust_model,                        // trust_model: TrustModel,
        false,                              // auto_init: bool,
        private,                            // private: bool,
        template,                           // template: bool,
        description.map(|x| x.to_string()), // description: Option<String>,
        None,                               // gitignores: Option<String>,
        None,                               // issue_labels: Option<String>,
        None,                               // license: Option<String>,
        None,                               // readme: Option<String>,
    )
}
fn suggested_remote_repo_name(local_path: &Path) -> String {
    // This assumes that the local_path has a file name,
    // and that it can be converted to Unicode
    local_path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn open_remote_url() {
        const REMOTE_NAME: &str = "origin";
        const TEST_PATH: &str = "C:\\Users\\psobo\\Development\\rust\\ckpath";
        // const REMOTE_URL: &str = "http://marconi/gitea/psobolik/ckpath-rust.git";

        let path = PathBuf::from(TEST_PATH);
        open_git_remote(REMOTE_NAME, Some(&path));
    }

    #[tokio::test]
    async fn create_repo() {
        const GITEA_URL: &str = "http://marconi/gitea";
        const USERNAME: &str = "psobolik";
        const PASSWORD: &str = "*****";

        let credentials = Credentials::with_url_username_password(GITEA_URL, USERNAME, PASSWORD);
        let options = CreateRepoOptions::new(
            "gitea-cli".to_string(), // name: String,
            "main".to_string(),      // default_branch: String,
            TrustModel::Default,     // trust_model: TrustModel,
            false,                   // auto_init: bool,
            false,                   // private: bool,
            false,                   // template: bool,
            None,                    // description: Option<String>,
            None,                    // gitignores: Option<String>,
            None,                    // issue_labels: Option<String>,
            None,                    // license: Option<String>,
            None,                    // readme: Option<String>,
        );
        match create_repository(GITEA_URL, &credentials, &options).await {
            Ok(repo) => println!("{:?}", repo),
            Err(error) => panic!("{:?}", error),
        }
    }
}
