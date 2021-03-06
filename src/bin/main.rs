extern crate argparse;
extern crate cntr;
extern crate nix;

use argparse::{ArgumentParser, Collect, List, Store};
use cntr::pwd::pwnam;
use std::io::{stderr, stdout};
use std::path::Path;
use std::str::FromStr;
use std::{env, process};

#[allow(non_camel_case_types)]
#[derive(Debug)]
enum Command {
    attach,
    exec,
}

impl FromStr for Command {
    type Err = ();
    fn from_str(src: &str) -> Result<Command, ()> {
        match src {
            "attach" => Ok(Command::attach),
            "exec" => Ok(Command::exec),
            _ => Err(()),
        }
    }
}

fn parse_attach_args(args: Vec<String>) -> cntr::AttachOptions {
    let mut options = cntr::AttachOptions {
        command: None,
        arguments: vec![],
        container_name: String::from(""),
        container_types: vec![],
        effective_user: None,
    };
    let mut container_type = String::from("");
    let mut container_name = String::from("");
    let mut effective_username = String::from("");
    let mut command = String::from("");
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Enter container");
        ap.refer(&mut effective_username).add_option(
            &["--effective-user"],
            Store,
            "effective username that should be owner of new created files on the host",
        );
        ap.refer(&mut container_type).add_option(
            &["-t", "--type"],
            Store,
            "Container type (process_id|rkt|docker|nspawn|lxc|lxd|command), default: all except command)",
        );
        ap.refer(&mut container_name).required().add_argument(
            "id",
            Store,
            "container id, container name or process id",
        );
        ap.refer(&mut command).add_argument(
            "command",
            Store,
            "command to execute after attach (default: $SHELL)",
        );
        ap.refer(&mut options.arguments).add_argument(
            "arguments",
            Collect,
            "arguments passed to command",
        );
        match ap.parse(args, &mut stdout(), &mut stderr()) {
            Ok(()) => {}
            Err(x) => {
                std::process::exit(x);
            }
        }
    }
    options.container_name = container_name;
    if !container_type.is_empty() {
        options.container_types = match cntr::lookup_container_type(container_type.as_str()) {
            Some(container) => vec![container],
            None => {
                eprintln!(
                    "invalid argument '{}' passed to `--type`; valid values are: {}",
                    container_type,
                    cntr::AVAILABLE_CONTAINER_TYPES.join(", ")
                );
                process::exit(1)
            }
        };
    }

    if effective_username != "" {
        match pwnam(effective_username.as_str()) {
            Ok(Some(passwd)) => {
                options.effective_user = Some(passwd);
            }
            Ok(None) => {
                eprintln!("no user with username '{}' found", effective_username);
                process::exit(1);
            }
            Err(e) => {
                eprintln!(
                    "failed to to lookup user '{}' found: {}",
                    effective_username, e
                );
                process::exit(1);
            }
        };
    }

    if command != "" {
        options.command = Some(command);
    }

    options
}

fn attach_command(args: Vec<String>) {
    let opts = parse_attach_args(args);
    if let Err(err) = cntr::attach(&opts) {
        eprintln!("{}", err);
        process::exit(1);
    };
}

fn exec_command(args: Vec<String>, setcap: bool) {
    let mut command = String::from("");
    let mut arguments = vec![];
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Execute command in container filesystem");
        ap.refer(&mut command).add_argument(
            &"command",
            Store,
            "command to execute (default: $SHELL)",
        );
        ap.refer(&mut arguments)
            .add_argument(&"arguments", List, "Arguments to pass to command");
        ap.stop_on_first_argument(true);
        match ap.parse(args, &mut stdout(), &mut stderr()) {
            Ok(()) => {}
            Err(x) => {
                std::process::exit(x);
            }
        }
    }
    let command = if command.is_empty() {
        None
    } else {
        Some(command)
    };

    if let Err(err) = cntr::exec(command, arguments, setcap) {
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn main() {
    match std::env::current_exe() {
        Ok(exe) => {
            if exe == Path::new(cntr::SETCAP_EXE) {
                exec_command(env::args().collect::<Vec<String>>(), true)
            }
        }
        Err(e) => {
            eprintln!("failed to resolve executable: {}", e);
            process::exit(1);
        }
    }

    let mut subcommand = Command::attach;
    let mut args = vec![];
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Enter or executed in container");
        ap.refer(&mut subcommand).required().add_argument(
            "command",
            Store,
            r#"Command to run (either "attach" or "exec")"#,
        );
        ap.refer(&mut args)
            .add_argument("arguments", List, r#"Arguments for command"#);

        ap.stop_on_first_argument(true);
        ap.parse_args_or_exit();
    }

    args.insert(0, format!("subcommand {:?}", subcommand));

    match subcommand {
        Command::attach => attach_command(args),
        Command::exec => exec_command(args, false),
    }
}
