use serde::Serialize;

#[derive(Serialize)]
pub struct CommandChoice<S: AsRef<str>> {
    pub name: S,
    pub value: S,
}

#[derive(Serialize)]
pub struct CommandOption<S: AsRef<str>> {
    pub name: S,
    pub description: S,
    #[serde(rename = "type")]
    pub option_type: u32,
    pub choices: Option<Vec<CommandChoice<S>>>,
    pub required: bool,
}

#[derive(Serialize)]
pub struct Command<S: AsRef<str>> {
    pub name: S,
    pub description: S,
    #[serde(rename = "type")]
    pub command_type: u16,
    pub options: Option<Vec<CommandOption<S>>>,
}

fn create_minecraft_command() -> Command<String> {
    Command {
        name: String::from("mc"),
        description: String::from("Minecraft slash commands"),
        options: Some(vec![CommandOption {
            option_type: 3,
            name: String::from("action"),
            description: String::from("Available actions"),
            required: true,
            choices: Some(vec![CommandChoice {
                name: String::from("Reboot"),
                value: String::from("reboot"),
            }]),
        }]),
        command_type: 1,
    }
}

pub fn create_all_commands() -> Vec<Command<String>> {
    vec![create_minecraft_command()]
}
