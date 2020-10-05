use std::ffi::OsString;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::io::{self, AsyncWriteExt};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::timeout;

use tokio::process::Command;

static sub_process_active: AtomicBool = AtomicBool::new(false);

pub fn register_ctrlc_handler() {
    ctrlc::set_handler(|| {
        if (!sub_process_active.load(Ordering::SeqCst)) {
            std::process::exit(137);
        }
    });
}

fn update_command<S: Into<String> + Clone>(
    command: &Vec<S>,
    srv_port: u16,
) -> Option<Vec<OsString>> {
    let lst_str: Vec<String> = command.iter().skip(1).map(|e| e.clone().into()).collect();

    let mut idx = 0;
    let mut do_continue = true;
    while idx < lst_str.len() && do_continue {
        if !lst_str[idx].starts_with("--") {
            do_continue = false
        } else {
            idx += 1
        }
    }

    if do_continue == true {
        return None;
    }

    let command_element: &str = &lst_str[idx].to_lowercase();

    match command_element {
        "build" => (),
        "test" => (),
        _ => return None,
    };

    let (pre_cmd, cmd_including_post) = lst_str.split_at(idx);
    let (cmd, post_command) = cmd_including_post.split_at(1);

    let bes_section = vec![
        cmd[0].clone(),
        String::from("--build_event_publish_all_actions"),
        String::from("--color"),
        String::from("yes"),
        String::from("--bes_backend"),
        String::from(format!("grpc://127.0.0.1:{}", srv_port)),
    ];

    Some(
        vec![pre_cmd.iter(), bes_section.iter(), post_command.iter()]
            .into_iter()
            .flat_map(|e| e)
            .map(|e| e.into())
            .collect(),
    )
}

#[derive(Clone, PartialEq, Debug)]
pub struct ExecuteResult {
    pub exit_code: i32,
    pub errors_corrected: u32,
}
pub async fn execute_bazel<S: Into<String> + Clone>(
    command: Vec<S>,
    bes_port: u16,
) -> ExecuteResult {
    let application: OsString = command
        .first()
        .map(|a| {
            let a: String = a.clone().into();
            a
        })
        .expect("Should have had at least one arg the bazel process itself.")
        .into();

    let updated_command = match update_command(&command, bes_port) {
        Some(e) => e,
        None => command
            .iter()
            .skip(1)
            .map(|str_ref| {
                let a: String = str_ref.clone().into();
                let a: OsString = a.into();
                a
            })
            .collect(),
    };

    println!("{:?} {:?}", application, updated_command);
    let mut cmd = Command::new(application);

    cmd.args(&updated_command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to start bazel process");

    let mut child_stdout = child.stdout.take().expect("Child didn't have a stdout");

    sub_process_active.store(true, Ordering::SeqCst);

    tokio::spawn(async move {
        let mut bytes_read = 1;
        let mut buffer = [0; 1024];
        let mut stdout = tokio::io::stdout();
        while bytes_read > 0 {
            bytes_read = child_stdout.read(&mut buffer[..]).await.unwrap();
            stdout.write_all(&buffer[0..bytes_read]).await.unwrap()
        }
    });

    let mut child_stderr = child.stderr.take().expect("Child didn't have a stderr");

    tokio::spawn(async move {
        let mut bytes_read = 1;
        let mut buffer = [0; 1024];
        let mut stderr = tokio::io::stderr();
        while bytes_read > 0 {
            bytes_read = child_stderr.read(&mut buffer[..]).await.unwrap();
            stderr.write_all(&buffer[0..bytes_read]).await.unwrap()
        }
    });
    let child_pid = child.id() as i32;

    let result = child.await.expect("The command wasn't running");

    sub_process_active.store(false, Ordering::SeqCst);

    ExecuteResult {
        exit_code: result.code().unwrap_or_else(|| -1),
        errors_corrected: 0,
    }
}
