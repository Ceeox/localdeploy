use std::{
    env,
    io::Write,
    path::PathBuf,
    process::{Child, Command},
    str::FromStr,
    thread,
    time::Duration,
};

use clap::{App, Arg, ArgMatches};
use error::Error;
use git2::{Cred, FetchOptions, RemoteCallbacks, Repository};

mod error;

use crate::error::Result;

fn main() -> Result<()> {
    let app = App::new("localdeploy")
        .version("0.1")
        .author("Ceeox <me@ceox.dev>")
        .arg(
            Arg::with_name("new")
                .short("n")
                .long("new")
                .takes_value(true)
                .value_name("NEW REPO")
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
            Arg::with_name("passphrase")
                .short("a")
                .long("passphrase")
                .takes_value(true)
                .value_name("PASSPHRASE")
                .help("Passphrase private ssl key. Nesassary for cloning a new repo"),
        )
        .arg(
            Arg::with_name("private-key")
                .short("k")
                .long("private-key")
                .takes_value(true)
                .value_name("PRIVATE KEY")
                .default_value("~/.ssh/id_rsa")
                .help("Path to the private ssl key. Nesassary for cloning a new Repository"),
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
        .get_matches();

    if app.is_present("new") {
        new_repo(&app)?;
        return Ok(());
    }

    daemon_run(&app)?;

    Ok(())
}

fn new_repo(app: &ArgMatches) -> Result<()> {
    let fo = fetch_options(app)?;
    // Prepare builder.
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);

    let path = match app.value_of("path") {
        None => env::current_dir()?,
        Some(p) => PathBuf::from_str(p).expect("Parsing PathBuf failed"),
    };

    let _ = std::fs::create_dir_all(path.clone())?;

    let _ = builder.clone(app.value_of("new").unwrap(), &path)?;
    Ok(())
}

fn fetch_options<'a>(app: &ArgMatches) -> Result<FetchOptions<'a>> {
    let private_key = if let Some(path) = app.value_of("private-key") {
        PathBuf::from_str(path).expect("Parsing PathBuf failed")
    } else {
        PathBuf::from_str(&format!("{}/.ssh/id_rsa", env::var("HOME")?))
            .expect("Parsing PathBuf failed")
    };
    let lock = std::io::stdout();
    print!("SSH Passphrase: ");
    let _ = lock.lock().flush();
    let passphrase = {
        let mut buffer = String::new();
        let handle = std::io::stdin();

        handle.read_line(&mut buffer)?;
        buffer.trim().to_owned()
    };
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(move |_url, username_from_url, _allowed_types| {
        Cred::ssh_key(
            username_from_url.unwrap(),
            None,
            &private_key,
            Some(&passphrase),
        )
    });

    // Prepare fetch options.
    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(callbacks);
    Ok(fo)
}

fn daemon_run(app: &ArgMatches) -> Result<()> {
    loop {
        let mut child: Child = routine(&app)?;

        let interval = match app.value_of("interval") {
            Some(r) => r.parse::<u64>().unwrap_or(3600),
            None => 3600,
        };
        thread::sleep(Duration::from_secs(interval));

        let _ = child.kill();
    }
}

fn routine(app: &ArgMatches) -> Result<Child> {
    let _repo = fetch_git_repo(&app)?;
    let full_cmd = match app.value_of("command") {
        Some(r) => r.to_owned(),
        None => return Err(Error::MissingCommand),
    };
    let mut args = full_cmd.trim().split(" ").collect::<Vec<&str>>();

    if args.len() <= 1 {
        return Err(Error::MissingCommand);
    }
    let cmd = args.remove(0);
    println!("cmd: {:#?}, args: {:#?}", cmd, args);
    let dir = repo_path(app)?;

    Ok(Command::new(cmd)
        .current_dir(dir)
        // .stdout(Stdio::piped())
        // .stdin(Stdio::piped())
        .args(args)
        .spawn()
        .expect("failed to spawn cmd"))
}

fn fetch_git_repo(app: &ArgMatches) -> Result<Repository> {
    let path = repo_path(app)?;
    let repo = Repository::discover(path)?;
    let origin = app.value_of("origin").unwrap_or("origin");
    let branch = app.value_of("branch").unwrap_or("main");
    let mut fo = fetch_options(app)?;
    repo.find_remote(origin)?
        .fetch(&[branch], Some(&mut fo), None)?;
    Ok(repo)
}

fn repo_path(app: &ArgMatches) -> Result<PathBuf> {
    println!("{:?}", app.value_of("path"));
    match app.value_of("path") {
        Some(path) => Ok(PathBuf::from_str(path).unwrap()),
        None => Ok(env::current_dir().expect("unable to get current dir")),
    }
}
