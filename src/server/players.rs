use oxford_join::OxfordJoin;

/// take two lists of player names and return the difference between them.
/// the first tuple is the list of players who have disconnected. the second tuple is of players who have joined.
pub fn get_player_diff(
    before: &[String],
    after: &[String],
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let disconnected = before
        .iter()
        .filter(|&player| !after.contains(player))
        .cloned()
        .collect();
    let mut joined = vec![];
    let mut remaining_online = vec![];
    for player in after {
        if !before.contains(player) {
            joined.push(player.clone());
        } else {
            remaining_online.push(player.clone());
        }
    }

    (disconnected, joined, remaining_online)
}

pub fn get_player_changes(before: &[String], after: &[String]) -> Option<String> {
    let (disconnected, joined, remaining) = get_player_diff(before, after);
    if disconnected.is_empty() && joined.is_empty() {
        return None;
    }
    // player1, player2, and player3 have joined. player4, player5, and player6 have disconnected
    Some(
        [
            if !joined.is_empty() {
                format!(
                    "{} {} joined.",
                    joined.oxford_join(oxford_join::Conjunction::And),
                    if joined.len() != 1 { "have" } else { "has" }
                )
            } else {
                "".to_string()
            },
            if !disconnected.is_empty() {
                format!(
                    "{} {} disconnected.",
                    disconnected.oxford_join(oxford_join::Conjunction::And),
                    if disconnected.len() != 1 {
                        "have"
                    } else {
                        "has"
                    }
                )
            } else {
                "".to_string()
            },
            if !remaining.is_empty() {
                format!(
                    "{} {} online.",
                    remaining.oxford_join(oxford_join::Conjunction::And),
                    if remaining.len() != 1 { "are" } else { "is" }
                )
            } else if !disconnected.is_empty() && joined.is_empty() {
                "Nobody is online.".to_string()
            } else {
                "".to_string()
            },
        ]
            .join(" "),
    )
}
