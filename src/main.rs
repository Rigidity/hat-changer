use std::{
    collections::HashMap,
    fs,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use colored::Colorize;
use go_parse_duration::parse_duration;
use pretty_duration::pretty_duration;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// An extremely lightweight time tracking tool for work.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// The name of the project.
    project_name: Option<String>,
}

#[derive(Parser, Debug)]
enum Commands {
    /// List all projects and their total time.
    List,

    /// Start the timer for the active project.
    On,

    /// Finish the active timer and log an entry.
    Off {
        /// The description of the logged time.
        #[arg(trailing_var_arg = true)]
        description: Vec<String>,
    },

    /// Edit the last logged time.
    Edit {
        /// The new duration of the last logged time.
        #[arg(trailing_var_arg = true)]
        duration: Vec<String>,
    },

    /// Undo the last logged time, or cancel the current entry.
    Undo,

    /// List all logged times for the active project.
    Time,

    /// Add a new project.
    New {
        /// The name of the project.
        project_name: String,
    },

    /// Delete a project.
    Delete {
        /// The name of the project.
        project_name: String,
    },
}

#[derive(Default, Serialize, Deserialize)]
struct ProjectList {
    projects: HashMap<String, Project>,
    active_project: Option<String>,
}

#[derive(Default, Serialize, Deserialize)]
struct Project {
    start_epoch: Option<Duration>,
    logged_times: Vec<LoggedTime>,
}

#[derive(Serialize, Deserialize)]
struct LoggedTime {
    start_epoch: Duration,
    duration: Duration,
    description: String,
}

#[derive(Debug, Error)]
enum Error {
    #[error("Could not parse duration with invalid format.")]
    ParseDuration(go_parse_duration::Error),

    #[error("An error occurred while trying to get the system's current time.")]
    SystemTime(#[from] std::time::SystemTimeError),

    #[error("There is no project named {}", .0.bright_cyan())]
    UnknownProject(String),

    #[error("You do not currently have a project selected.")]
    NoActiveProject,

    #[error("The active project does not exist anymore.")]
    UnknownActiveProject,

    #[error("You are already tracking your time.")]
    AlreadyStarted,

    #[error("You have not started tracking your time.")]
    NotStarted,

    #[error("You have not logged any time for this project.")]
    NoTimeLogged,

    #[error("Cannot log entry with no description.")]
    NoDescription,

    #[error("project {} already exists", .0.bright_cyan())]
    ProjectExists(String),
}

type Result<T> = std::result::Result<T, Error>;

fn main() {
    let args = Args::parse();

    let home = homedir::get_my_home()
        .expect("Could not read home directory.")
        .expect("Home directory not found.");

    let path = home.join(".timelogger.json");

    let mut list: ProjectList = fs::read_to_string(path.as_path())
        .map(|text| serde_json::from_str(&text).unwrap())
        .unwrap_or_default();

    let result = match args.command {
        Some(Commands::List) => handle_list(&list),
        Some(Commands::On) => handle_on(&mut list),
        Some(Commands::Off { description }) => handle_off(&mut list, &description.join(" ")),
        Some(Commands::Edit { duration }) => handle_edit(&mut list, &duration.join(" ")),
        Some(Commands::Undo) => handle_undo(&mut list),
        Some(Commands::Time) => handle_time(&list),
        Some(Commands::New { project_name }) => handle_new(&mut list, &project_name),
        Some(Commands::Delete { project_name }) => handle_delete(&mut list, &project_name),
        None => {
            if let Some(project_name) = args.project_name {
                handle_hat(&mut list, &project_name)
            } else {
                handle_time(&list)
            }
        }
    };

    if let Err(err) = result {
        eprintln!("{}", err.to_string().bright_yellow());
    }

    fs::write(
        path.as_path(),
        serde_json::to_string_pretty(&list).expect("Could not serialize JSON file."),
    )
    .expect("Could not write JSON file.");
}

fn handle_list(list: &ProjectList) -> Result<()> {
    if list.projects.is_empty() {
        println!("{}", "No projects found.".bright_red());
        return Ok(());
    } else {
        println!("{}", "Project list:".bright_yellow());
    }
    for (name, project) in list.projects.iter() {
        let name = if list.active_project == Some(name.clone()) {
            name.bright_green()
        } else {
            name.bright_cyan()
        };

        let time = project
            .logged_times
            .iter()
            .fold(Duration::default(), |acc, time| acc + time.duration);

        let time = pretty_duration(&time, None).bright_red();

        println!("  {name} - {time}");
    }

    Ok(())
}

fn handle_on(list: &mut ProjectList) -> Result<()> {
    let Some(active) = list.active_project.clone() else {
        return Err(Error::NoActiveProject);
    };

    let Some(project) = list.projects.get_mut(&active) else {
        return Err(Error::UnknownActiveProject);
    };

    if project.start_epoch.is_some() {
        return Err(Error::AlreadyStarted);
    }

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
    project.start_epoch = Some(now);

    let name = active.bright_cyan();

    println!(
        "{}",
        format!("Now tracking time for project {}.", name).bright_green()
    );

    Ok(())
}

fn handle_off(list: &mut ProjectList, description: &str) -> Result<()> {
    let Some(active) = list.active_project.clone() else {
        return Err(Error::NoActiveProject);
    };

    let Some(project) = list.projects.get_mut(&active) else {
        return Err(Error::UnknownActiveProject);
    };

    if description.trim().is_empty() {
        return Err(Error::NoDescription);
    }

    let Some(start_epoch) = project.start_epoch.take() else {
        return Err(Error::NotStarted);
    };

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let duration = now - start_epoch;

    project.logged_times.push(LoggedTime {
        start_epoch,
        duration,
        description: description.trim().to_string(),
    });

    let name = active.bright_cyan();
    let time = pretty_duration(&duration, None).bright_red();

    println!(
        "{}",
        format!("Logged {} for project {}.", time, name).bright_green()
    );

    Ok(())
}

fn handle_edit(list: &mut ProjectList, duration: &str) -> Result<()> {
    let Some(active) = list.active_project.clone() else {
        return Err(Error::NoActiveProject);
    };

    let Some(project) = list.projects.get_mut(&active) else {
        return Err(Error::UnknownActiveProject);
    };

    let Some(time) = project.logged_times.last_mut() else {
        return Err(Error::NoTimeLogged);
    };

    let duration = Duration::from_nanos(
        parse_duration(&duration.replace(' ', "")).map_err(Error::ParseDuration)? as u64,
    );

    let old_duration = pretty_duration(&time.duration, None).bright_red();
    time.duration = duration;

    let duration = pretty_duration(&duration, None).bright_red();

    println!(
        "{}",
        format!("Modified the last entry from {old_duration} to {duration}").bright_green()
    );

    Ok(())
}

fn handle_undo(list: &mut ProjectList) -> Result<()> {
    let Some(active) = list.active_project.clone() else {
        return Err(Error::NoActiveProject);
    };

    let Some(project) = list.projects.get_mut(&active) else {
        return Err(Error::UnknownActiveProject);
    };

    if let Some(start) = project.start_epoch {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
        let duration = now - start;
        let time = pretty_duration(&duration, None).bright_red();

        project.start_epoch = None;

        println!(
            "{}",
            format!("Cancelled {time} of unlogged time.").bright_green()
        );

        return Ok(());
    }

    let Some(time) = project.logged_times.pop() else {
        return Err(Error::NoTimeLogged);
    };

    let description = time.description.bright_blue();
    let time = pretty_duration(&time.duration, None).bright_red();

    println!(
        "{}",
        format!("Removed the last entry with duration {time}: {description}").bright_green()
    );

    Ok(())
}

fn handle_time(list: &ProjectList) -> Result<()> {
    let Some(active) = list.active_project.clone() else {
        return Err(Error::NoActiveProject);
    };

    let Some(project) = list.projects.get(&active) else {
        return Err(Error::UnknownActiveProject);
    };

    let name = active.bright_cyan();

    if project.logged_times.is_empty() {
        println!(
            "{}",
            format!("No logged times for project {}.", name).bright_red()
        );
        return Ok(());
    }

    let total_duration = project
        .logged_times
        .iter()
        .fold(Duration::default(), |acc, time| acc + time.duration);
    let total = pretty_duration(&total_duration, None).bright_red();

    println!(
        "{}",
        format!("Logged times for {name}, totaling {total}:").bright_yellow()
    );

    for logged_time in project.logged_times.iter() {
        let time = pretty_duration(&logged_time.duration, None).bright_red();
        let description = logged_time.description.bright_blue();

        println!("  {time} - {description}");
    }

    Ok(())
}

fn handle_new(list: &mut ProjectList, name: &str) -> Result<()> {
    if list.projects.contains_key(name) {
        return Err(Error::ProjectExists(name.to_string()));
    }

    list.projects.insert(name.to_string(), Project::default());
    list.active_project = Some(name.to_string());

    let name = name.bright_cyan();

    println!("{}", format!("Added project {name}").bright_green());

    Ok(())
}

fn handle_delete(list: &mut ProjectList, name: &str) -> Result<()> {
    if list.projects.remove(name).is_some() {
        let name = name.bright_cyan();
        println!("{}", format!("Removed project {name}").bright_green());
    } else {
        return Err(Error::UnknownProject(name.to_string()));
    }

    if list.active_project == Some(name.to_string()) {
        list.active_project = None;
    }

    Ok(())
}

fn handle_hat(list: &mut ProjectList, name: &str) -> Result<()> {
    if list.projects.contains_key(name) {
        list.active_project = Some(name.to_string());
        let name = name.bright_cyan();
        println!("{}", format!("Selected project {name}").bright_green());
    } else {
        return Err(Error::UnknownProject(name.to_string()));
    }

    Ok(())
}
