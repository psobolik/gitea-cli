mod error;

pub extern crate colored;

use crate::error::CliError;
use clap::{command, Arg, ArgAction, ArgMatches, Command};
use git_lib::credentials::Credentials;
use git_lib::GitLib;
use gitea_api::models::create_repo_options::CreateRepoOptions;
use gitea_api::models::repository::Repository;
use gitea_api::models::trust_model::TrustModel;
use gitea_api::GiteaApi;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

#[macro_export]
macro_rules! print_info {
        ($($arg:tt)*) => {{
            let message = format!($($arg)*);
            println!("{} {message}", $crate::colored::Colorize::bright_green("->"))
        }};
    }

#[macro_export]
macro_rules! print_error {
        ($($arg:tt)*) => {{
            let message = format!($($arg)*);
            eprintln!("{} {message}", $crate::colored::Colorize::bright_red("->"))
        }};
    }

#[tokio::main]
async fn main() {
    let matches = get_clap_builder_command().get_matches();

    match matches.subcommand() {
        Some(("repo", repo_sub_matches)) => match repo_sub_matches.subcommand() {
            Some(("browse", browse_sub_matches)) => {
                do_browse(browse_sub_matches);
            }
            Some(("create", create_sub_matches)) => match do_create(create_sub_matches).await {
                Ok(repository) => {
                    print_info!("Created remote repository: {}", repository.clone_url);
                    match do_add_remote(create_sub_matches, repository.clone_url.as_str()) {
                        Ok(remote_name) => {
                            print_info!("Tracking remote repository locally as: {}", remote_name);
                            print_info!(
                                "Push: git push -u {} {}",
                                remote_name,
                                repository.default_branch
                            );
                        }
                        Err(error) => print_error!("{}", error),
                    }
                }
                Err(error) => print_error!("{}", error),
            },
            _ => unreachable!("Unexpected repo subcommand"),
        },
        _ => unreachable!("Unexpected subcommand"),
    }
}

fn do_browse(sub_matches: &ArgMatches) {
    let name = sub_matches
        .get_one::<String>("remote")
        .expect("Missing remote name");
    let path: Option<PathBuf> = sub_matches.get_one::<String>("path").map(PathBuf::from);
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
        .long("gitea_name");

    let default_branch_arg = Arg::new("default_branch")
        .help("Default branch")
        .short('b')
        .long("branch")
        .default_value("main");

    let description_arg = Arg::new("description")
        .help("Description")
        .short('d')
        .long("description");

    let issue_labels_arg = Arg::new("issue_labels")
        .help("Issue labels")
        .long("issue-labels");

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

    command!().arg_required_else_help(true).subcommand(
        Command::new("repo")
            .about("Work with Gitea repositories")
            .arg_required_else_help(true)
            .subcommand(Command::new("browse").arg(&remote_arg).arg(&path_arg))
            .subcommand(
                Command::new("create")
                    .arg(path_arg)
                    .arg(gitea_url_arg)
                    .arg(description_arg)
                    .arg(gitea_name_arg)
                    .arg(default_branch_arg)
                    .arg(remote_arg)
                    .arg(private_arg)
                    .arg(template_arg)
                    .arg(trust_model_arg)
                    .arg(issue_labels_arg),
            ),
    )
}

async fn do_create(matches: &ArgMatches) -> Result<Repository, CliError> {
    // Make sure the Gitea server URL is known
    let url = match matches.get_one::<String>("url") {
        Some(url) => url,
        None => return Err(error::CliError::from("Missing Gitea URL")),
    };
    // Get the top-level path, and make sure it is inside a repository.
    // (If there's no path, this uses the current folder.)
    let path: Option<PathBuf> = matches.get_one::<String>("path").map(PathBuf::from);
    let top_level = match GitLib::top_level(path.as_ref()) {
        Ok(top_level) => top_level,
        Err(error) => return Err(error::CliError::from(error)),
    };
    // Get the credentials for the Gitea server
    let credentials = match GitLib::credentials_fill(url) {
        Ok(credentials) => credentials,
        Err(error) => return Err(error::CliError::from(error)),
    };
    // If the Gitea repository name hasn't been specified, calculate it from the path
    let gitea_name = match matches.get_one::<String>("gitea_name") {
        Some(gitea_name) => gitea_name.to_string(),
        _ => suggested_remote_repo_name(&top_level),
    };
    let options = repo_options(gitea_name.as_str(), matches);
    create_repository(url, &credentials, &options).await
}

fn do_add_remote(matches: &ArgMatches, clone_url: &str) -> Result<String, CliError> {
    let remote = matches
        .get_one::<String>("remote")
        .expect("Missing remote name");
    let path: Option<PathBuf> = matches.get_one::<String>("path").map(PathBuf::from);
    if let Err(error) = GitLib::remote_add(remote.as_ref(), clone_url, path.as_ref()) {
        Err(error::CliError::from(error))
    } else {
        Ok(remote.to_string())
    }
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

fn open_git_remote(repo: &str, path: Option<&PathBuf>) {
    match GitLib::remote_url(repo, path) {
        Ok(remote_url) => {
            let ru = <String as AsRef<OsStr>>::as_ref(&remote_url);
            match open::that_detached(ru) {
                Ok(()) => println!("Opened '{}'", remote_url),
                Err(error) => print_error!("Error opening '{}': {}", remote_url, error),
            }
        }
        Err(error) => print_error!("Error getting remote URL for '{}': {}", repo, error),
    }
}

fn repo_options(name: &str, matches: &ArgMatches) -> CreateRepoOptions {
    let description = matches.get_one::<String>("description");
    let issue_labels = matches.get_one::<String>("issue_labels");
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
        println!("Issue labels:   {:?}", issue_labels.map(|x| x.to_string()));
        println!("Trust model:    {}", trust_model);
    */
    CreateRepoOptions::new(
        name.to_string(),                    // name: String,
        default_branch.to_string(),          // default_branch: String,
        trust_model,                         // trust_model: TrustModel,
        false,                               // auto_init: bool,
        private,                             // private: bool,
        template,                            // template: bool,
        description.map(|x| x.to_string()),  // description: Option<String>,
        None,                                // gitignores: Option<String>,
        issue_labels.map(|x| x.to_string()), // issue_labels: Option<String>,
        None,                                // license: Option<String>,
        None,                                // readme: Option<String>,
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
