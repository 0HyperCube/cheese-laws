#[macro_use]
extern crate log;

extern crate serde;

mod net;
use std::{collections::HashSet, fs::File, io::BufReader, iter::Peekable, str::Chars};

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

async fn format_content<'a>(client: &mut LawsClient, mut content: Peekable<Chars<'a>>) -> String {
	fn parse_id<'a>(content: &mut Peekable<Chars<'a>>) -> String {
		let mut id = String::new();
		while let Some(c) = content.next() {
			if c == '>' {
				break;
			} else {
				id.push(c);
			}
		}
		id
	}

	let mut bold = false;
	let mut italic = false;
	let mut result = String::new();
	while let Some(ch) = content.next() {
		match ch {
			'*' => {
				if content.peek() == Some(&'*') {
					content.next();
					result += if bold { "</b>" } else { "<b>" };
					bold = !bold;
				} else {
					result += if italic { "</i>" } else { "<i>" };
					italic = !italic;
				}
			}
			'<' => match content.next() {
				Some('#') => {
					let id = parse_id(&mut content);
					result += r#"<span class="ping">#"#;
					result += client.get_channel_name(&id).await.unwrap();
					result += "</span>";
				}
				Some('@') => {
					if Some(&'!') == content.peek() {
						content.next();
					}
					let id = parse_id(&mut content);
					result += r#"<span class="ping">@"#;
					info!("ID {}", &id);
					result += client.get_nickname(&id).await.unwrap();
					result += "</span>";
				}
				Some(v) => {
					result.push('<');
					result.push(v);
				}
				None => {}
			},
			'\n' => result += "<br>",
			_ => result.push(ch),
		}
	}
	result
}

async fn get_messages(client: &mut LawsClient, thread: &Thread) -> Result<(String, bool, String), LawError> {
	let msgs: Vec<Message> = client
		.request(format!("{}/channels/{}/messages?limit=100", LawsClient::API, thread.id))
		.await?
		.decode()
		.unwrap();
	let (mut users_for, mut users_against) = (HashSet::new(), HashSet::new());
	let mut description = String::new();
	for message in msgs.into_iter().rev() {
		let content = format_content(client, message.content.chars().peekable()).await;

		description += "<b>";
		description += client.get_nickname(&message.author.id).await.unwrap();
		description += ":</b> ";
		description += &content;
		description += "<br>";

		let lower_content = content.to_lowercase();
		if lower_content.contains("against") {
			users_for.remove(&message.author.id);
			users_against.insert(message.author.id);
		} else if lower_content.contains("for") {
			users_against.remove(&message.author.id);
			users_for.insert(message.author.id);
		}
	}
	let passed = users_for.len() > users_against.len();
	let votes = format!("{}-for-{}-against", users_for.len(), users_against.len());

	Ok((description, passed, votes))
}

async fn update_info(client: &mut LawsClient, thread: Thread, current: Option<&LawInfo>, constitution: &str) -> LawInfo {
	let ((description, passed, votes), interpretation) = if let Some(current) = current {
		if current.last_message_id != thread.last_message_id {
			(get_messages(client, &thread).await.unwrap(), current.interpretation.clone())
		} else {
			(
				(current.description.clone(), current.passed, current.votes.clone()),
				current.interpretation.clone(),
			)
		}
	} else {
		(get_messages(client, &thread).await.unwrap(), String::new())
	};

	let status = match (thread.thread_metadata.archived, passed) {
		(true, true) => "Passed",
		(true, false) => "Not passed",
		(false, _) => "Voting",
	}
	.to_string();

	LawInfo {
		id: thread.id,
		last_message_id: thread.last_message_id,
		name: thread.name,
		votes,
		passed,
		status,
		constitution: thread.parent_id == constitution,
		interpretation,
		description,
	}
}

async fn run() {
	let path = "web/laws.json";
	let mut laws_data: LawData = match File::open(path) {
		Ok(f) => {
			let reader = BufReader::new(f);
			serde_json::from_reader(reader).unwrap_or(LawData::default())
		}
		Err(_) => LawData::default(),
	};

	let mut client = LawsClient::new();

	#[allow(unused_variables)]
	let (test, parliament, constitution) = ("910596571509456959", "907664196567703584", "907661773925126164");

	let threads = get_threads(&client, &[parliament, constitution], &["archived/public", "active"])
		.await
		.unwrap();

	let mut last_index = -1;
	for t in threads {
		let index = laws_data.laws.iter_mut().position(|l| l.id == t.id);

		if let Some(index) = index {
			laws_data.laws[index] = update_info(&mut client, t, Some(&laws_data.laws[index]), constitution).await;
			last_index = index as isize;
		} else {
			last_index += 1;
			laws_data
				.laws
				.insert((last_index) as usize, update_info(&mut client, t, None, constitution).await)
		}
	}

	laws_data.generated = chrono::offset::Local::now().format("%a %e %b").to_string();

	let file = File::create(path).unwrap();

	let formatter = serde_json::ser::PrettyFormatter::with_indent(b"	");
	let mut ser = serde_json::Serializer::with_formatter(file, formatter);
	laws_data.serialize(&mut ser).unwrap();
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
