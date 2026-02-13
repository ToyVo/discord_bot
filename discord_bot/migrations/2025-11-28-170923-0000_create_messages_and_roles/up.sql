-- Your SQL goes here
CREATE TABLE "self_assignable_roles"(
	"id" UUID NOT NULL PRIMARY KEY,
	"emoji" VARCHAR NOT NULL,
	"guild_id" VARCHAR NOT NULL,
	"role_id" VARCHAR NOT NULL
);

CREATE TABLE "reaction_messages"(
	"id" UUID NOT NULL PRIMARY KEY,
	"message_type" VARCHAR NOT NULL,
	"guild_id" VARCHAR NOT NULL,
	"role_id" VARCHAR NOT NULL
);

