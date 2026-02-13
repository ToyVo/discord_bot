// @generated automatically by Diesel CLI.

diesel::table! {
    reaction_messages (id) {
        guild_id -> Varchar,
        id -> Uuid,
        message_id -> Varchar,
        message_type -> Varchar,
    }
}

diesel::table! {
    self_assignable_roles (id) {
        emoji -> Varchar,
        guild_id -> Varchar,
        id -> Uuid,
        role_id -> Varchar,
    }
}

diesel::allow_tables_to_appear_in_same_query!(reaction_messages, self_assignable_roles,);
