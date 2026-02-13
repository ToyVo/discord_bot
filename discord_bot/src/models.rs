use crate::schema::{reaction_messages, self_assignable_roles};
use diesel::prelude::*;
use uuid::Uuid;

#[derive(HasQuery)]
pub struct SelfAssignableRole {
    pub emoji: String,
    pub guild_id: String,
    pub id: Uuid,
    pub role_id: String,
}

#[derive(Insertable)]
#[diesel(table_name = self_assignable_roles)]
pub struct NewSelfAssignableRole {
    pub emoji: String,
    pub guild_id: String,
    pub role_id: String,
}

#[derive(HasQuery)]
pub struct ReactionMessage {
    pub guild_id: String,
    pub id: Uuid,
    pub message_id: String,
    pub message_type: String,
}

#[derive(Insertable)]
#[diesel(table_name = reaction_messages)]
pub struct NewReactionMessage {
    pub guild_id: String,
    pub message_id: String,
    pub message_type: String,
}
