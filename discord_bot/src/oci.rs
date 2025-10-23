use {axum::http::header, reqwest::Method, serde_json::json};

pub async fn restart_instance<S: AsRef<str>>(s: S) -> Result<(), Error> {
    let region = "us-chicago-1";
    let instance_id = "ocid1.instance.oc1.us-chicago-1.anxxeljreamwweaczolj43ykq53jdlavbyliuvfav2x3vs4dcp7prih5wlva";
    let url = format!(
        "https://iaas.{region}.oraclecloud.com/20160918/instances/{instance_id}?action=RESET"
    );

    let mut builder = reqwest::Client::new()
        .request(Method::POST, url.as_str())
        .header(
            header::CONTENT_TYPE.as_str(),
            mime::APPLICATION_JSON.as_ref(),
        )
        .json(&json!({
            "actionType": "reset"
        }));

    let response = builder.send().await?;

    tracing::debug!("response from {method} {url}: {response:?}");
    let content_type = response.headers().get(header::CONTENT_TYPE.as_str());
    if content_type.is_some()
        && content_type.unwrap().to_str().unwrap() == mime::APPLICATION_JSON.as_ref()
    {
        let body = response.json::<Value>().await?;
        tracing::debug!("response body from {method} {url}: {body}");
        return Ok(Some(body));
    }

    Ok(None)
}
