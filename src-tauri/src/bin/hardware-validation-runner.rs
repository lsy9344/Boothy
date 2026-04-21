use std::{path::PathBuf, process::ExitCode};

use boothy_lib::automation::hardware_validation::{
    default_runtime_base_dir, run_hardware_validation_in_dir, AppLaunchMode,
    HardwareValidationRunInput,
};

fn main() -> ExitCode {
    match parse_args(std::env::args().skip(1).collect()) {
        Ok(config) => run(config),
        Err(message) => {
            eprintln!("{message}");
            print_usage();
            ExitCode::from(2)
        }
    }
}

struct CliConfig {
    prompt: String,
    preset_query: String,
    capture_count: u32,
    phone_last_four: Option<String>,
    base_dir: PathBuf,
    output_dir: PathBuf,
    app_launch_mode: AppLaunchMode,
}

fn run(config: CliConfig) -> ExitCode {
    let result = run_hardware_validation_in_dir(
        &config.base_dir,
        &config.output_dir,
        HardwareValidationRunInput {
            prompt: config.prompt,
            preset_query: config.preset_query,
            capture_count: config.capture_count,
            app_launch_mode: config.app_launch_mode,
            phone_last_four: config.phone_last_four,
        },
    );

    match result {
        Ok(run_result) => {
            println!("status={}", run_result.status);
            println!("run_dir={}", run_result.run_dir.display());
            println!("summary={}", run_result.summary_path.display());
            if let Some(path) = run_result.failure_report_path.as_deref() {
                println!("failure_report={}", path.display());
            }

            if run_result.status == "passed" {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn parse_args(args: Vec<String>) -> Result<CliConfig, String> {
    let mut prompt = None;
    let mut preset_query = "look2".to_string();
    let mut capture_count = 5_u32;
    let mut phone_last_four = None;
    let mut base_dir = None;
    let mut output_dir = None;
    let mut app_launch_mode = AppLaunchMode::LaunchSiblingExe;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--prompt" => {
                index += 1;
                prompt = args.get(index).cloned();
            }
            "--preset" => {
                index += 1;
                preset_query = args
                    .get(index)
                    .cloned()
                    .ok_or_else(|| "--preset requires a value".to_string())?;
            }
            "--capture-count" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--capture-count requires a value".to_string())?;
                capture_count = value
                    .parse::<u32>()
                    .map_err(|_| "--capture-count must be a positive integer".to_string())?
                    .max(1);
            }
            "--phone-last-four" => {
                index += 1;
                phone_last_four = Some(
                    args.get(index)
                        .cloned()
                        .ok_or_else(|| "--phone-last-four requires a value".to_string())?,
                );
            }
            "--base-dir" => {
                index += 1;
                base_dir = Some(PathBuf::from(
                    args.get(index)
                        .cloned()
                        .ok_or_else(|| "--base-dir requires a value".to_string())?,
                ));
            }
            "--output-dir" => {
                index += 1;
                output_dir =
                    Some(PathBuf::from(args.get(index).cloned().ok_or_else(
                        || "--output-dir requires a value".to_string(),
                    )?));
            }
            "--skip-app-launch" => {
                app_launch_mode = AppLaunchMode::Skip;
            }
            "--launch-app" => {
                app_launch_mode = AppLaunchMode::LaunchSiblingExe;
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            unknown => {
                return Err(format!("Unknown argument: {unknown}"));
            }
        }
        index += 1;
    }

    let prompt = prompt.ok_or_else(|| "--prompt is required".to_string())?;
    let base_dir = base_dir.unwrap_or_else(default_runtime_base_dir);
    let output_dir = output_dir.unwrap_or_else(|| base_dir.join("hardware-validation-runs"));

    Ok(CliConfig {
        prompt,
        preset_query,
        capture_count,
        phone_last_four,
        base_dir,
        output_dir,
        app_launch_mode,
    })
}

fn print_usage() {
    eprintln!(
        "Usage: hardware-validation-runner --prompt <text> [--preset look2] [--capture-count 5] [--phone-last-four 4821] [--base-dir <path>] [--output-dir <path>] [--skip-app-launch]"
    );
}
