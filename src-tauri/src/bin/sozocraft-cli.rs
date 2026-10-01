#[tokio::main]
async fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(sozocraft_lib::cli::run(std::env::args_os().skip(1)).await)
}
