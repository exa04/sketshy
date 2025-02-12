use std::{
    fs::{exists, metadata, read_dir},
    path::{Path, PathBuf},
};

use crate::action::{Action, Action::*};

type Completer = fn(&str) -> Vec<String>;
fn completer_path(input: &str) -> Vec<String> {
    let path = PathBuf::from(format!("./{}", input));

    let read_all = |path: &Path| {
        read_dir(path)
            .ok()
            .map(|files| files.filter_map(|file| file.ok()))
    };

    let mut v = if !exists(&path).is_ok_and(|e| e) {
        let file_name = path
            .file_stem()
            .map(|n| n.to_ascii_lowercase().as_encoded_bytes().to_owned())
            .unwrap_or_default();

        path.parent()
            .and_then(|parent| {
                read_all(parent).map(|f| {
                    f.filter(|f| {
                        f.file_name()
                            .to_ascii_lowercase()
                            .as_encoded_bytes()
                            .starts_with(&file_name)
                    })
                    .map(|f| {
                        let mut name = f.path().to_string_lossy().into_owned().split_off(2);
                        if f.metadata().is_ok_and(|m| m.is_dir()) {
                            name.push('/');
                        }
                        name
                    })
                    .collect::<Vec<_>>()
                })
            })
            .unwrap_or_default()
    } else if metadata(&path).is_ok_and(|m| m.is_dir()) {
        read_all(&path)
            .map(|f| {
                f.map(|f| {
                    let mut name = f.path().to_string_lossy().into_owned().split_off(2);
                    if f.metadata().is_ok_and(|m| m.is_dir()) {
                        name.push('/');
                    }
                    name
                })
                .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    } else if metadata(&path).is_ok_and(|m| m.is_file() | m.is_symlink()) {
        vec![path.to_str().unwrap_or_default().to_owned().split_off(2)]
    } else {
        vec![]
    };

    v.sort();
    v
}

struct Command {
    name: &'static str,
    aliases: &'static [&'static str],
    description: &'static str,
    args: &'static [Completer],
    action: fn(&[&str]) -> Option<Action>,
}

#[derive(Clone)]
pub struct Completion {
    pub val: String,
    pub description: Option<String>,
    pub full: String,
}

pub fn get_completions(input: &str) -> Vec<Completion> {
    let mut input_split = input.split(' ').peekable();

    if let Some(ident) = input_split.next().filter(|i| !i.is_empty()) {
        if input_split.peek().is_some() {
            if let Some(command) = parse_ident(ident) {
                input_split
                    .enumerate()
                    .last()
                    .and_then(|(i, arg)| {
                        command.args.get(i).map(|completer| {
                            let head = input.split_whitespace().take(i + 1).collect::<String>();
                            completer(arg)
                                .into_iter()
                                .map(|c| Completion {
                                    val: c.clone(),
                                    description: None,
                                    full: format!("{} {}", head, c),
                                })
                                .collect::<Vec<_>>()
                        })
                    })
                    .unwrap_or_default()
            } else {
                vec![]
            }
        } else {
            COMMANDS
                .iter()
                .filter_map(|c| {
                    [c.aliases, &[c.name]]
                        .concat()
                        .iter()
                        .any(|id| id.starts_with(ident))
                        .then_some(Completion {
                            val: c.name.to_string(),
                            description: Some(if c.aliases.is_empty() {
                                c.description.to_owned()
                            } else {
                                format!("{}. Aliases: {}", c.description, c.aliases.join(", "))
                            }),
                            full: c.name.to_string(),
                        })
                })
                .collect::<Vec<_>>()
        }
    } else {
        COMMANDS
            .iter()
            .map(|c| Completion {
                val: c.name.to_string(),
                description: Some(if c.aliases.is_empty() {
                    c.description.to_owned()
                } else {
                    format!("{}. Aliases: {}", c.description, c.aliases.join(", "))
                }),
                full: c.name.to_string(),
            })
            .collect::<Vec<_>>()
    }
}

fn parse_ident(ident: &str) -> Option<&Command> {
    COMMANDS
        .iter()
        .find(|c| c.name == ident || c.aliases.contains(&ident))
}

pub fn parse_command(input: &str) -> Option<Action> {
    let mut input = input.split_whitespace();

    input
        .next()
        .and_then(|ident| parse_ident(ident))
        .map(|command| (command, input.collect::<Vec<_>>()))
        .filter(|(command, args)| args.len() == command.args.len())
        .and_then(|(command, args)| (command.action)(&args))
}

const COMMANDS: &[Command] = &[
    Command {
        name: "quit",
        aliases: &["q"],
        description: "Quit sketshy",
        args: &[],
        action: |_args| -> Option<Action> { Some(Quit) },
    },
    Command {
        name: "export",
        aliases: &["e"],
        description: "Export to a plaintext file",
        args: &[completer_path],
        action: |args| -> Option<Action> { Some(Export(args[0].to_string())) },
    },
    // Command {
    //     name: "import",
    //     aliases: &["i"],
    //     description: "Import from a plaintext file",
    //     args: &[completer_path],
    //     action: |args| -> Option<Action> { Some(Import(args[0].to_string())) },
    // },
];

// impl Command {
//     pub fn parse(input: &str) -> Option<Self> {
//         match input.split_whitespace().collect::<Vec<_>>().as_slice() {
//             _ => None,
//         }
//     }
//     pub fn get_completion(input: &str) -> Vec<String> {
//         let mut input = input.split_whitespace().peekable();
//         let ident = input.next();
//         if input.peek().is_none() {
//             todo!()
//         } else {
//             vec![]
//         }
//     }
//     pub fn description(&self) -> &'static str {
//         match self {
//             Command::Export(_) => "Export to plaintext",
//             Command::Import(_) => "Import from plaintext",
//             Command::Quit => "Quit sketshy",
//             Command::Foo(_, _) => "Bar",
//         }
//     }
//     pub fn run(&self) -> Option<Action> {
//         match self {
//             Command::Quit => Some(Action::Quit),
//             _ => todo!(),
//         }
//     }
// }
