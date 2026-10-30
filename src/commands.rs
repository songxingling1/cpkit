use crate::{
    cli::{CompileCommand, InitCommand, RunCommand},
    compile::Artifact,
    configuration::{Checker, ProblemConfig, ProblemType},
    print_progress,
};
use anyhow::{Context as _, Result, ensure};
use cpkit_task::{Task, TaskResult};
use std::path::{Path, PathBuf};

use crate::context::{Context, ProblemContext};

/// 统一的 problem context 加载
async fn load_problem_context(ctx: &Context, source: &Path) -> Result<ProblemContext> {
    ensure!(
        source.is_file(),
        "Source file not found: {}",
        source.display()
    );
    let mut pctx = ctx
        .get_problem_context(source)
        .context("Failed to load problem context.")?;
    pctx.load_config()
        .await
        .context("Failed to load problem configuration.")?;
    Ok(pctx)
}

/// Windows 上自动补 `.exe`，其它平台原样返回。
fn with_exe_suffix(path: PathBuf) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let mut path = path;
        path.set_extension("exe");
    }
    path
}

/// 用当前配置编译 problem，返回产物路径。
async fn build_artifact(
    ctx: &Context,
    pctx: &ProblemContext,
    output: Option<PathBuf>,
) -> Result<PathBuf> {
    let config = ctx
        .get_config()
        .context("Configuration not loaded; this indicates a bug.")?;
    let sources = pctx
        .get_source_list()
        .context("Source list not available; this indicates a bug.")?;
    Artifact::builder()
        .source(sources)
        .compiler(config.compile.compiler.clone())
        .maybe_output(output.map(with_exe_suffix))
        .options(
            config
                .compile
                .flag
                .iter()
                .map(Clone::clone)
                .map(Into::into)
                .collect(),
        )
        .compile()
        .await
        .context("Compilation failed.")
}

/// 承接编译请求。
pub async fn compile(
    ctx: &crate::context::Context,
    CompileCommand { source, output }: CompileCommand,
) -> Result<()> {
    let pctx = load_problem_context(ctx, &source).await?;
    print_progress!("Compiling", "{}", source.display());
    let artifact = build_artifact(ctx, &pctx, output).await?;
    print_progress!(
        "Finished",
        "Successfully compiled {} to {}.",
        source.display(),
        artifact.display()
    );
    Ok(())
}

/// 承接运行请求
pub async fn run(ctx: &crate::context::Context, RunCommand { source }: RunCommand) -> Result<()> {
    let pctx = load_problem_context(ctx, &source).await?;
    let artifact = with_exe_suffix(
        ctx.get_cache_dir().join(
            pctx.get_hash()
                .await
                .context("Failed to compute problem hash.")?,
        ),
    );
    if !artifact.is_file() {
        print_progress!("Compiling", "{}", source.display());
        build_artifact(ctx, &pctx, Some(artifact.clone())).await?;
    }
    #[cfg(unix)]
    let mut ctrl_c = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    #[cfg(windows)]
    let mut ctrl_c = tokio::signal::windows::ctrl_c()?;
    let mut tasks = cpkit_task::TaskSet::new();
    let cancel = tasks
        .spawn(Task::builder().id(1).executable(artifact.clone()).build())
        .context("Failed to spawn executable.")?;
    print_progress!("Running", "{} ({})", artifact.display(), source.display());
    let result = tokio::select! {
        res = tasks.next() => res,
        _ = ctrl_c.recv() => {
            cancel.cancel();
            tasks.next().await
        }
    };
    let TaskResult { task: _, usage } = result.context("Failed to get result.")?;
    print_progress!(
        "Finished",
        "Execution time: {}  Peak memory: {} MiB  Exit signal: {}.",
        humantime::format_duration(usage.time),
        usage.memory,
        usage.status
    );
    Ok(())
}

pub async fn init(
    ctx: &crate::context::Context,
    InitCommand {
        source,
        with_checker,
        with_interactive,
    }: InitCommand,
) -> Result<()> {
    ensure!(
        source.is_file(),
        "Source file not found: {}",
        source.display()
    );
    let pctx = ctx
        .get_problem_context(&source)
        .context("Failed to load problem context.")?;

    let mut config = ProblemConfig::default();
    if with_checker {
        config.checker = Checker::Special {
            path: PathBuf::new(),
        };
    }
    if with_interactive {
        config.problem_type = ProblemType::Interactive {
            header: PathBuf::new(),
            grader: PathBuf::new(),
        };
    }
    pctx.write_problem_config(&config)
        .await
        .context("Failed to write problem configuration.")?;
    print_progress!(
        "Finished",
        "Created configuration for {}.",
        source.display()
    );
    Ok(())
}
