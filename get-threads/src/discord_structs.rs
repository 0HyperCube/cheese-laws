use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ThreadMetadata {
	pub archived: bool,
	pub archive_timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct Thread {
	pub id: String,
	pub name: String,
	pub last_message_id: String,
	pub thread_metadata: ThreadMetadata,
	pub parent_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ThreadList {
	pub threads: Vec<Thread>,
	pub has_more: bool,
}
#[derive(Debug, Deserialize)]
pub struct Author {
	pub id: String,
}
#[derive(Debug, Deserialize)]
pub struct Message {
	pub content: String,
	pub author: Author,
}
#[derive(Debug, Deserialize)]
pub struct GuildUser {
	pub username: String,
}
#[derive(Debug, Deserialize)]
pub struct GuildMember {
	pub nick: Option<String>,
	pub user: GuildUser,
}

#[derive(Debug, Deserialize)]
pub struct Channel {
	pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LawInfo {
	pub id: String,
	pub last_message_id: String,
	pub name: String,
	pub votes: String,
	pub passed: bool,
	pub constitution: bool,
	pub status: String,
	pub interpretation: String,
	pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LawData {
	pub generated: String,
	pub laws: Vec<LawInfo>,
}
