#[macro_use]
extern crate log;

extern crate serde;

mod net;
use std::{collections::HashSet, fs::File, io::BufReader};

use net::*;

mod discord_structs;
use discord_structs::*;
use serde::Serialize;

async fn get_threads(client: &LawsClient, channels: &[&str], thread_types: &[&str]) -> Result<Vec<Thread>, LawError> {
	let mut thread_list = Vec::new();
	for channel in channels {
		for endpoint in thread_types {
			let mut before = String::new();
			loop {
				let threads = client
					.request(format!("{}/channels/{}/threads/{}{}", LawsClient::API, channel, endpoint, before))
					.await?
					.decode::<ThreadList>()?;
				thread_list.extend(threads.threads);
				if !threads.has_more {
					break;
				}
				before = format!(
					"?before={}",
					thread_list.last().unwrap().thread_metadata.archive_timestamp.split("+").next().unwrap()
				);
			}
		}
	}

	info!("Enumerated {:?} threads", thread_list.len());

	thread_list.sort_unstable_by(|a, b| b.thread_metadata.archive_timestamp.cmp(&a.thread_metadata.archive_timestamp));
	Ok(thread_list)
}

async fn make_info(client: &mut LawsClient, thread: Thread) -> Result<LawInfo, LawError> {
	let msgs: Vec<Message> = client
		.request(format!("{}/channels/{}/messages?limit=100", LawsClient::API, thread.id))
		.await?
		.decode()
		.unwrap();
	let (mut users_for, mut users_against) = (HashSet::new(), HashSet::new());
	let mut description = String::new();
	for message in msgs.into_iter().rev() {
		description += "<b>";
		description += client.get_nickname(&message.author.id).await.unwrap();
		description += "</b>";
		description += &message.content.replace("\n", "<br>");
		description += "<br>";

		let lower_content = message.content.to_lowercase();
		if lower_content.contains("for") {
			users_against.remove(&message.author.id);
			users_for.insert(message.author.id);
		} else if lower_content.contains("against") {
			users_for.remove(&message.author.id);
			users_against.insert(message.author.id);
		}
	}
	let passed = users_for.len() > users_against.len();
	let result = match (thread.thread_metadata.archived, passed) {
		(true, true) => "Passed",
		(true, false) => "Not passed",
		(false, _) => "Voting",
	};
	let status = format!("{}: {} for, {} against", result, users_for.len(), users_against.len());

	Ok(LawInfo {
		id: thread.id,
		name: thread.name,
		last_message_id: thread.last_message_id,
		status,
		interpretation: String::new(),
		description,
	})
}

async fn run() {
	let path = "web/laws.json";
	let mut laws: Vec<LawInfo> = match File::open(path) {
		Ok(f) => {
			let reader = BufReader::new(f);
			serde_json::from_reader(reader).unwrap_or(Vec::new())
		}
		Err(_) => Vec::new(),
	};

	let mut client = LawsClient::new();

	#[allow(unused_variables)]
	let (test, parliament, constitution) = ("910596571509456959", "907664196567703584", "907661773925126164");

	let threads = get_threads(&client, &[parliament], &["archived/public", "active"]).await.unwrap();

	for t in threads {
		let index = laws.iter_mut().find(|l| l.id == t.id);

		if let Some(index) = index {
			if t.last_message_id != index.last_message_id {
				let info = make_info(&mut client, t).await.unwrap();
				index.last_message_id = info.last_message_id;
				index.name = info.name;
				index.status = info.status;
				index.description = info.description;
			}
		} else {
			let info = make_info(&mut client, t).await.unwrap();
			laws.push(info);
		}
	}

	let file = File::create(path).unwrap();

	let formatter = serde_json::ser::PrettyFormatter::with_indent(b"	");
	let mut ser = serde_json::Serializer::with_formatter(file, formatter);
	laws.serialize(&mut ser).unwrap();
}

// Use simplelog with a file and the console.
fn init_logger() {
	use simplelog::*;

	CombinedLogger::init(vec![
		TermLogger::new(LevelFilter::Debug, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
		WriteLogger::new(LevelFilter::Debug, Config::default(), File::create("CheeseBot.log").unwrap()),
	])
	.unwrap();
}

fn main() {
	init_logger();

	tokio::runtime::Builder::new_current_thread()
		.enable_all()
		.build()
		.unwrap()
		.block_on(run());
}
