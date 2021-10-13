use std::{
    env,
    path::PathBuf,
    process::{Child, Command, Stdio},
    str::FromStr,
    thread,
    time::Duration,
};

use clap::{App, Arg, ArgMatches};
use error::Error;
use git2::{Cred, FetchOptions, RemoteCallbacks, Repository};
use rpassword::prompt_password_stdout;

mod error;

use crate::error::Result;

pub(crate) struct Main {
    origin: String,
    branch: String,
    cmd: String,
    args: Vec<String>,
    repo_path: PathBuf,
    child: Option<Child>,
    repo: Option<Repository>,
    interval: u64,
    username: String,
    public_key_path: PathBuf,
    private_key_path: PathBuf,
    passphrase: Option<String>,
}

impl Main {
    pub fn new(app: ArgMatches) -> Result<Self> {
        let origin = app.value_of("origin").unwrap_or("origin").to_owned();
        let branch = app.value_of("branch").unwrap_or("main").to_owned();
        let command = match app.value_of("command") {
            Some(r) => r.to_owned(),
            None => return Err(Error::MissingCommand),
        };
        let repo_path = match app.value_of("path") {
            Some(path) => PathBuf::from_str(path).unwrap().to_owned(),
            None => env::current_dir()?.to_owned(),
        };

        let public_key_path = if let Some(path) = app.value_of("public-key") {
            PathBuf::from_str(path).expect("Parsing PathBuf failed")
        } else {
            PathBuf::from_str(&format!("{}/.ssh/id_rsa.pub", env::var("HOME")?))
                .expect("Parsing PathBuf failed")
        };
        let private_key_path = if let Some(path) = app.value_of("private-key") {
            PathBuf::from_str(path).expect("Parsing PathBuf failed")
        } else {
            PathBuf::from_str(&format!("{}/.ssh/id_rsa", env::var("HOME")?))
                .expect("Parsing PathBuf failed")
        };
        let interval = match app.value_of("interval") {
            Some(r) => r.parse::<u64>().unwrap_or(3600),
            None => 3600,
        };
        let username = app.value_of("username").unwrap_or("").to_owned();
        let (cmd, args) = Main::parse_cmd_args(command)?;

        let mut _self = Self {
            child: None,
            branch,
            origin,
            cmd,
            args,
            repo_path,
            repo: None,
            interval,
            username,
            public_key_path,
            private_key_path,
            passphrase: None,
        };

        if app.is_present("use-passphrase") {
            _self.passphrase()
        }
        let repo = match (app.is_present("new"), app.is_present("path")) {
            (true, true) => {
                let new = match app.value_of("new") {
                    Some(new) => new,
                    None => return Err(Error::MissingUrlToRepo),
                };
                Main::new_repo(new, _self.fetch_options(), &_self.repo_path)?
            }
            (true, false) => return Err(Error::MissingPath),
            (false, true) => Repository::discover(_self.repo_path.clone())?,

            (false, false) => return Err(Error::MissingPath),
        };
        _self.repo = Some(repo);

        Ok(_self)
    }

    pub fn new_repo<'fo>(
        new: &str,
        fetch_options: FetchOptions<'fo>,
        path: &PathBuf,
    ) -> Result<Repository> {
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);
        let _ = std::fs::create_dir_all(path.clone())?;
        Ok(builder.clone(new, &path)?)
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            let _repo = self.fetch_git_repo()?;
            self.spawn_cmd()?;
            thread::sleep(Duration::from_secs(self.interval));

            if let Some(child) = &mut self.child {
                let _ = child.kill();
            }
        }
    }

    fn spawn_cmd(&mut self) -> Result<()> {
        self.child = Some(
            Command::new(self.cmd.clone())
                .current_dir(self.repo_path.clone())
                .stdout(Stdio::piped())
                .stdin(Stdio::piped())
                .args(self.args.clone())
                .spawn()
                .expect("failed to spawn cmd"),
        );
        Ok(())
    }

    fn fetch_git_repo(&mut self) -> Result<()> {
        let mut fo = self.fetch_options();

        if let Some(repo) = &self.repo {
            repo.find_remote(&self.origin)?
                .fetch(&[self.branch.clone()], Some(&mut fo), None)?;
        }
        Ok(())
    }

    fn fetch_options(&self) -> FetchOptions<'_> {
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(move |_url, username_from_url, _allowed_types| {
            let username = if let Some(u) = username_from_url {
                u
            } else {
                &self.username
            };
            let mut cred = Cred::ssh_key_from_agent(username);
            if cred.is_err() {
                cred = Cred::ssh_key(
                    username_from_url.unwrap(),
                    Some(&self.public_key_path),
                    &self.private_key_path,
                    self.passphrase.as_deref(),
                );
            }
            cred
        });

        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);
        fetch_options
    }

    fn passphrase(&mut self) {
        self.passphrase = Some(prompt_password_stdout("SSH Passphrase: ").unwrap_or("".to_owned()));
    }

    fn parse_cmd_args(command: String) -> Result<(String, Vec<String>)> {
        let mut args = command
            .trim()
            .split(" ")
            .map(|s| s.to_owned())
            .collect::<Vec<String>>();

        if args.len() <= 1 {
            return Err(Error::MissingCommand);
        }
        let cmd = args.remove(0);
        Ok((cmd.to_owned(), args))
    }
}

fn main() -> Result<()> {
    let app = App::new("localdeploy")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Ceeox <me@ceox.dev>")
        .arg(
            Arg::with_name("new")
                .short("n")
                .long("new")
                .takes_value(true)
                .value_name("REPO_URL")
                .help(
                    "Url to the new git repo. Ensure a path to where the repo should to cloned to.",
                ),
        )
        .arg(
            Arg::with_name("branch")
                .short("b")
                .long("branch")
                .takes_value(true)
                .value_name("BRANCH")
                .default_value("main")
                .help("Provides a default branch to fetch repo from"),
        )
        .arg(
            Arg::with_name("remote")
                .short("r")
                .long("remote")
                .takes_value(true)
                .value_name("REMOTE")
                .default_value("origin")
                .help("Provides a default origin to fetch repo from"),
        )
        .arg(
            Arg::with_name("public-key")
                .long("public-key")
                .takes_value(true)
                .value_name("PUBLIC_KEY")
                .default_value("~/.ssh/id_rsa.pub")
                .help("Path to the public ssl key"),
        )
        .arg(
            Arg::with_name("private-key")
                .long("private-key")
                .takes_value(true)
                .value_name("PRIVATE_KEY")
                .default_value("~/.ssh/id_rsa")
                .help("Path to the private ssl key"),
        )
        .arg(
            Arg::with_name("path")
                .short("p")
                .long("path")
                .takes_value(true)
                .value_name("PATH")
                .help("File path to the existing repo"),
        )
        .arg(
            Arg::with_name("command")
                .short("c")
                .long("command")
                .takes_value(true)
                .value_name("CMD")
                .help("Command to run the project"),
        )
        .arg(
            Arg::with_name("interval")
                .short("i")
                .long("interval")
                .takes_value(true)
                .value_name("INTERVAL")
                .default_value("3600")
                .help("Interval between each git fetch in sec"),
        )
        .arg(
            Arg::with_name("username")
                .short("u")
                .long("username")
                .takes_value(true)
                .value_name("USERNAME")
                .default_value("git")
                .help("Username for git auth"),
        )
        .arg(
            Arg::with_name("use-passphrase")
                .short("s")
                .long("use-passphrase")
                .help("Give a hint if the ssh private is protected by a passphrase"),
        )
        .get_matches();

    let mut main = Main::new(app)?;
    main.run()?;

    Ok(())
}
