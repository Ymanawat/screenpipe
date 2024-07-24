// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dirs::home_dir;
use log::{debug, error, info, LevelFilter};
use logs::MultiWriter;

use serde_json::Value;
use std::fs::File;
use std::io::Write;

use std::fs;
use std::path::PathBuf;

use tauri::Manager;
use tauri::Wry;
use tauri_plugin_store::{with_store, StoreCollection};

use tauri_plugin_autostart::MacosLauncher;
mod analytics;

use crate::analytics::start_analytics;
mod find_screenpipe;
mod logs;

fn get_base_dir(custom_path: Option<String>) -> anyhow::Result<PathBuf> {
    let default_path = home_dir()
        .ok_or("Failed to get home directory")
        .unwrap()
        .join(".screenpipe");

    let local_data_dir = custom_path.map(PathBuf::from).unwrap_or(default_path);

    fs::create_dir_all(&local_data_dir.join("data"))?;
    Ok(local_data_dir)
}



#[tokio::main]
async fn main() {
    let _guard = sentry::init(("https://cf682877173997afc8463e5ca2fbe3c7@o4507617161314304.ingest.us.sentry.io/4507617170161664", sentry::ClientOptions {
        release: sentry::release_name!(),
        ..Default::default()
      }));

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        // .plugin(tauri_plugin_cli::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(move |app| {
            // let cli = app.cli().matches().expect("Failed to get CLI matches");

            let base_dir = get_base_dir(None).expect("Failed to ensure local data directory");
            let port = 3030;

            app.manage(port);

            let debug = true;

            let mut builder = env_logger::Builder::new();
            builder
                .filter(None, LevelFilter::Info)
                .filter_module("tokenizers", LevelFilter::Error)
                // .filter_module("rusty_tesseract", LevelFilter::Error)
                .filter_module("symphonia", LevelFilter::Error);

            if debug {
                builder.filter_module("screenpipe", LevelFilter::Debug);
                builder.filter_module("app", LevelFilter::Debug);
            }

            // debug!("all param: {:?}", cli.args);

            let log_file =
                File::create(format!("{}/screenpipe-app.log", base_dir.to_string_lossy())).unwrap();
            let multi_writer = MultiWriter::new(vec![
                Box::new(log_file) as Box<dyn Write + Send>,
                Box::new(std::io::stdout()) as Box<dyn Write + Send>,
            ]);

            builder.target(env_logger::Target::Pipe(Box::new(multi_writer)));
            builder.format_timestamp_secs().init();

            info!("Local data directory: {}", base_dir.display());

            let posthog_api_key = "phc_Bt8GoTBPgkCpDrbaIZzJIEYt0CrJjhBiuLaBck1clce".to_string();
            let app_name = "screenpipe";
            let interval_hours = 1;

            let path = base_dir.join("store.bin");

            if !path.exists() {
                let _ = File::create(path.clone()).unwrap();
            }

            let stores = app.app_handle().state::<StoreCollection<Wry>>();

            let _ = with_store(app.app_handle().clone(), stores, path, |store| {
                store.save()?;

                let is_analytics_enabled = store
                    .get("analytics_enabled")
                    .unwrap_or(&Value::Bool(true))
                    .as_bool()
                    .unwrap_or(true);

                if is_analytics_enabled {
                    match start_analytics(posthog_api_key, app_name, interval_hours) {
                        Ok(analytics_manager) => {
                            app.manage(analytics_manager);
                        }
                        Err(e) => {
                            error!("Failed to start analytics: {}", e);
                        }
                    }
                }

                Ok(())
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // Run the app
    app.run(|_app_handle, event| match event {
        tauri::RunEvent::Ready { .. } => {
            debug!("Ready event");
            tauri::async_runtime::spawn(async move {
                let _ = start_server().await;
            });
        }
        tauri::RunEvent::ExitRequested { .. } => {
            debug!("ExitRequested event");
            // tauri::async_runtime::spawn(async move {
            //     tx.send(()).unwrap();
            // });
            // TODO less dirty stop :D
            tauri::async_runtime::spawn(async move {
                let _ = tokio::process::Command::new("pkill")
                    .arg("-f")
                    .arg("screenpipe")
                    .output()
                        .await;
            });
        }
        _ => {}
    });
}

use tokio::io::{AsyncBufReadExt, BufReader};

async fn start_server() -> anyhow::Result<()> {
    let _t = tokio::spawn(async move {
        debug!("Starting server");

        // kill any all process on port 3030
        let _ = tokio::process::Command::new("pkill")
            .arg("-f")
            .arg("screenpipe")
            .output()
            .await;

        // // print path
        // debug!("Path: {:?}", std::env::var("PATH").unwrap());

        // // set PATH to all stuf in ./*
        // std::env::set_var(
        //     "PATH",
        //     format!("{:?}:{:?}", std::env::var("PATH").unwrap(), "."),
        // );

        // // also add /Applications/screenpipe.app/Contents/Resources
        // std::env::set_var(
        //     "PATH",
        //     format!(
        //         "{:?}:{:?}",
        //         std::env::var("PATH").unwrap(),
        //         "/Applications/screenpipe.app/Contents/Resources/tesseract"
        //     ),
        // );
        // std::env::set_var(
        //     "PATH",
        //     format!(
        //         "{:?}:{:?}",
        //         std::env::var("PATH").unwrap(),
        //         "/Applications/screenpipe.app/Contents/Resources"
        //     ),
        // );

        // debug!("Path: {:?}", std::env::var("PATH").unwrap());

        // list files ./
        // debug!(
        //     "Files: {:?}",
        //     std::fs::read_dir(".").unwrap().collect::<Vec<_>>()
        // );

        let mut cmd =
            tokio::process::Command::new(find_screenpipe::find_screenpipe_path().unwrap());
        cmd.arg("--port").arg("3030");
        cmd.arg("--debug");
        // let base_dir = get_base_dir(None).expect("Failed to ensure local data directory");
        // cmd.arg("--data-dir").arg(base_dir);
        // add tesseract to path from ./tesseract
        // cmd.env(
        //     "PATH",
        //     format!(
        //         "{:?}:{:?}",
        //         std::env::var("PATH").unwrap(),
        //         "/Applications/screenpipe.app/Contents/Resources/tesseract"
        //     ),
        // );
        // cmd.env(
        //     "PATH",
        //     format!(
        //         "{:?}:{:?}",
        //         std::env::var("PATH").unwrap(),
        //         "/Applications/screenpipe.app/Contents/Resources"
        //     ),
        // );
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().expect("Failed to spawn command");

        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let stderr = child.stderr.take().expect("Failed to capture stderr");

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        tokio::spawn(async move {
            while let Ok(Some(line)) = stdout_reader.next_line().await {
                debug!("{}", line);
            }
        });

        tokio::spawn(async move {
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                debug!("{}", line);
            }
        });

        let output = child
            .wait_with_output()
            .await
            .expect("Failed to wait for child process");
        debug!("Child process exited with output: {:?}", output);
        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Child process exited with output: {:?}",
                output
            ))
        }
    });

    // stop when rx is received
    // let _ = rx.recv().await;
    // t.abort();

    Ok(())
}