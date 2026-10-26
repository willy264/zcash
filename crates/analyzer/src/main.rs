use analyzer::{
    analyze, enforce, render_report, require_exercised, GroundTruth, PairReport, RunInput,
};
use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, ValueEnum)]
enum Expect {
    Leaky,
    Clean,
}

#[derive(Debug, Parser)]
#[command(about = "Check captured wallet privacy evidence and render the Darkroom report")]
struct Args {
    #[arg(long, default_value = ".runs")]
    runs: PathBuf,
    #[arg(long, default_value = "fixtures/ground_truth.json")]
    ground_truth: PathBuf,
    #[arg(long, default_value = "report")]
    output: PathBuf,
    #[arg(long, value_enum)]
    expect: Option<Expect>,
    #[arg(long)]
    observations: Option<PathBuf>,
    #[arg(long)]
    app_log: Option<PathBuf>,
    #[arg(long)]
    wallet: Option<PathBuf>,
    #[arg(long, default_value = "A1,A3,A2", value_delimiter = ',')]
    expected_leaks: Vec<String>,
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("cannot read required evidence {}", path.display()))
}

fn input(obs: &Path, app: &Path, wallet: &Path) -> Result<RunInput> {
    RunInput::parse(&read(obs)?, &read(app)?, &read(wallet)?)
}

fn main() -> Result<()> {
    let args = Args::parse();
    fs::create_dir_all(&args.output)?;
    // Invalidate success entrypoints before any evidence read. A failed direct
    // invocation must not leave yesterday's green report looking current.
    for name in ["index.html", "result.json"] {
        let path = args.output.join(name);
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("invalidate prior report {}", path.display()))
            }
        }
    }
    let gt: GroundTruth =
        serde_json::from_str(&read(&args.ground_truth)?).context("invalid ground truth JSON")?;
    gt.validate()?;
    if let Some(expect) = args.expect {
        let obs = args
            .observations
            .context("--expect requires --observations")?;
        let app = args.app_log.context("--expect requires --app-log")?;
        let wallet = args.wallet.context("--expect requires --wallet")?;
        let result = analyze(&input(&obs, &app, &wallet)?, &gt)?;
        fs::write(
            args.output.join("result.json"),
            serde_json::to_vec_pretty(&result)?,
        )?;
        require_exercised(&result, &args.expected_leaks)?;
        let expected = match expect {
            Expect::Leaky => args.expected_leaks,
            Expect::Clean => vec![],
        };
        enforce(&result, &expected)?;
        println!(
            "Verified {}: {} observed finding(s).",
            result.profile,
            result.leaked_ids().len()
        );
        return Ok(());
    }
    if args.observations.is_some() || args.app_log.is_some() || args.wallet.is_some() {
        bail!("individual evidence paths require --expect; otherwise use --runs");
    }
    let mut results = Vec::new();
    for profile in ["leaky", "fixed"] {
        let dir = args.runs.join(profile);
        let obs = dir.join("observations.jsonl");
        let app = dir.join("app.log");
        let wallet = dir.join("wallet.json");
        let result = analyze(&input(&obs, &app, &wallet)?, &gt)
            .with_context(|| format!("{profile} evidence validation failed"))?;
        for (source, suffix) in [
            (&obs, "observations.jsonl"),
            (&app, "app.log"),
            (&wallet, "wallet.json"),
        ] {
            fs::copy(source, args.output.join(format!("{profile}-{suffix}")))?;
        }
        results.push(result);
    }
    let fixed = results.pop().expect("fixed result");
    let leaky = results.pop().expect("leaky result");
    let report = PairReport::new(&gt, leaky, fixed, args.expected_leaks.clone());
    fs::write(
        args.output.join("result.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    fs::write(args.output.join("index.html"), render_report(&report))?;
    println!("Report: {}", args.output.join("index.html").display());
    enforce(&report.leaky, &args.expected_leaks)
        .context("leaky run did not reproduce the expected findings")?;
    enforce(&report.fixed, &[]).context("fixed run has a privacy regression")?;
    require_exercised(&report.fixed, &args.expected_leaks)
        .context("fixed run skipped an expected check")?;
    if !report.passed {
        bail!("leaky and fixed runs must cover the identical scan window and send scenario");
    }
    println!(
        "PASS: leaky reproduced [{}]; fixed has zero observed findings.",
        args.expected_leaks.join(", ")
    );
    Ok(())
}
