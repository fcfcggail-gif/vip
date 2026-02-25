//! Auto-Updater Daemon — به‌روزرسانی خودکار لیست IPها

use std::time::Duration;
use anyhow::{Context, Result};
use tokio::time::timeout;
use tracing::{error, info, warn};

const LOCK_FILE: &str = "/tmp/network-ghost-updater.lock";
const PROXY_FILE: &str = "/opt/network-ghost/sub/proxies.txt";
const LOG_DIR: &str = "/opt/network-ghost/logs";
const MAX_RUN_SECS: u64 = 600;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    info!("🔄 Auto-Updater v5.0 شروع شد");

    // بررسی قفل
    if !acquire_lock().await? {
        warn!("⚠️ نمونه دیگری در حال اجراست — خروج.");
        return Ok(());
    }

    let result = timeout(
        Duration::from_secs(MAX_RUN_SECS),
        run_update()
    ).await;

    // آزاد کردن قفل
    tokio::fs::remove_file(LOCK_FILE).await.ok();

    match result {
        Ok(Ok(())) => {
            info!("✅ به‌روزرسانی موفق");
            let ts = chrono::Local::now().to_string();
            tokio::fs::write(format!("{}/last-success.txt", LOG_DIR), ts).await.ok();
        }
        Ok(Err(e)) => error!("❌ خطا: {}", e),
        Err(_) => error!("❌ timeout پس از {} ثانیه", MAX_RUN_SECS),
    }
    Ok(())
}

async fn acquire_lock() -> Result<bool> {
    let lock_path = std::path::Path::new(LOCK_FILE);
    if lock_path.exists() {
        // بررسی سن قفل
        if let Ok(meta) = tokio::fs::metadata(LOCK_FILE).await {
            if let Ok(modified) = meta.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    if elapsed.as_secs() < 660 {
                        return Ok(false);
                    }
                }
            }
        }
    }
    tokio::fs::write(LOCK_FILE, std::process::id().to_string()).await?;
    Ok(true)
}

async fn run_update() -> Result<()> {
    // بررسی اتصال اینترنت
    if !check_internet().await {
        warn!("⚠️ اتصال اینترنت در دسترس نیست");
        return Ok(());
    }

    // ایجاد دایرکتوری‌ها
    tokio::fs::create_dir_all("/opt/network-ghost/sub").await.ok();
    tokio::fs::create_dir_all(LOG_DIR).await.ok();
    tokio::fs::create_dir_all("/opt/network-ghost/cache").await.ok();

    // اجرای proxy-checker
    let checker_path = which_binary("proxy-checker");
    if let Some(checker) = checker_path {
        info!("🔍 اجرای proxy-checker...");
        let status = tokio::process::Command::new(&checker)
            .args(&[
                "--max-concurrent", "30",
                "--timeout", "6",
                "--max-ping-ms", "300",
                "--output-file", &format!("{}/proxies-report.md", LOG_DIR),
            ])
            .status()
            .await
            .context("proxy-checker اجرا نشد")?;

        if status.success() {
            info!("✅ proxy-checker کامل شد");
        } else {
            warn!("⚠️ proxy-checker با کد {} خارج شد", status.code().unwrap_or(-1));
        }
    } else {
        warn!("⚠️ proxy-checker یافت نشد");
    }

    Ok(())
}

async fn check_internet() -> bool {
    use tokio::net::TcpStream;
    use tokio::time::timeout;
    let targets = ["1.1.1.1:443", "8.8.8.8:443", "9.9.9.9:443"];
    for target in &targets {
        if timeout(Duration::from_secs(3), TcpStream::connect(target)).await.is_ok() {
            return true;
        }
    }
    false
}

fn which_binary(name: &str) -> Option<String> {
    let paths = [
        format!("/opt/network-ghost/{}", name),
        format!("/usr/local/bin/{}", name),
        format!("./{}", name),
    ];
    for p in &paths {
        if std::path::Path::new(p).exists() {
            return Some(p.clone());
        }
    }
    None
}
