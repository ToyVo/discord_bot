use {
    chrono::prelude::*,
    dioxus::prelude::*,
    std::time::Duration,
};

const VALID_SERVICES: [&str; 3] = [
    "arion-minecraft-modded.service",
    "arion-minecraft-geyser.service",
    "arion-terraria.service",
];

#[component]
pub fn Logs() -> Element {
    let mut logs = use_signal(String::new);
    let unit = use_signal(|| String::from("arion-minecraft-modded.service"));
    let now = Utc::now();
    let since = now - Duration::from_secs(3600);
    let until = now;
    rsx! {
        form {
            label {
                r#for: "unit-select",
                "Unit"
            }
            select {
                id: "unit-select",
                name: "unit",
                for service in VALID_SERVICES {
                    option {
                        value: service,
                        selected: service == unit.read().as_str(),
                        {service}
                    }
                }
            }
            label {
                r#for: "since-input",
                "Since"
            }
            input {
                id: "since-input",
                name: "since",
                r#type: "datetime-local",
                value: since.format("%Y-%m-%dT%H:%M:%S").to_string(),
            }
            label {
                r#for: "until-input",
                "Until"
            }
            input {
                id: "until-input",
                name: "until",
                r#type: "datetime-local",
                value: until.format("%Y-%m-%dT%H:%M:%S").to_string(),
            }
            small {
                "Note: time shown in UTC"
            }
            button {
                onclick: move |_| async move {
                    if let Ok(new_logs) = fetch_logs(unit.to_string(), since.to_rfc3339(), until.to_rfc3339()).await {
                        logs.set(new_logs);
                    }
                },
                r#type: "button",
                "Fetch Logs"
            }
        }
        pre {
            margin: 0,
            {logs}
        }
    }
}

#[server]
async fn fetch_logs(unit: String, since: String, until: String) -> Result<String, ServerFnError> {
    if !VALID_SERVICES.contains(&unit.as_str()) {
        return Err(ServerFnError::Args(String::from("invalid unit")));
    }
    let journalctl_args = [
        "--utc",
        "-u",
        unit.as_str(),
        "-S",
        since.as_str(),
        "-U",
        until.as_str(),
    ];
    tracing::info!("fetching logs for unit: {unit}");
    Ok(journalctl_args.join(" "))
}
