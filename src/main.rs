fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("mneme=info".parse().unwrap_or_default()),
        )
        .with_target(false)
        .compact()
        .init();

    if let Err(err) = mneme::cli::run() {
        eprintln!("mneme error: {err:#}");
        std::process::exit(1);
    }
}
