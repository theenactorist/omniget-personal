use anyhow::anyhow;

pub async fn resolve_redirect(client: &reqwest::Client, url: &str) -> anyhow::Result<String> {
    crate::core::personal_policy::require_url(url).map_err(anyhow::Error::msg)?;

    let response = client.get(url).send().await?;

    let final_url = response.url().to_string();

    crate::core::personal_policy::require_url(&final_url).map_err(anyhow::Error::msg)?;

    if final_url == url {
        return Err(anyhow!("Nenhum redirect encontrado para {}", url));
    }

    Ok(final_url)
}
