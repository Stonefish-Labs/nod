#[cfg(target_os = "windows")]
pub(crate) fn open_url(url: &str) -> anyhow::Result<()> {
    std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn()?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub(crate) fn open_url(url: &str) -> anyhow::Result<()> {
    std::process::Command::new("xdg-open").arg(url).spawn()?;
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub(crate) fn open_url(_url: &str) -> anyhow::Result<()> {
    // Keep development builds portable while making unsupported runtime behavior explicit.
    anyhow::bail!("opening links is only supported on Windows and Linux")
}

#[cfg(all(test, not(any(target_os = "windows", target_os = "linux"))))]
mod tests {
    use super::*;

    #[test]
    fn unsupported_platform_returns_clear_error() {
        let error = open_url("https://nod.example.com").unwrap_err();

        assert!(error.to_string().contains("Windows and Linux"));
    }
}
