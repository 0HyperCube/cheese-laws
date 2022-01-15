use std::{collections::HashMap, str::Utf8Error};

use hyper::{
	body::Bytes,
	client::HttpConnector,
	header::{HeaderValue, AUTHORIZATION},
	Body, Client, Method, Request, StatusCode,
};
use hyper_tls::HttpsConnector;
use serde::Deserialize;

use crate::discord_structs::{Channel, GuildMember};

#[derive(std::fmt::Debug)]
pub enum LawError {
	Hyper(hyper::Error),
	Utf8(Utf8Error),
	DeJson(serde_json::Error),
}

#[allow(dead_code)]
pub struct Response {
	status: StatusCode,
	bytes: Bytes,
}
impl Response {
	pub fn utf(&self) -> Result<&str, LawError> {
		let utf = std::str::from_utf8(&self.bytes).map_err(|e| LawError::Utf8(e))?;
		Ok(utf)
	}
	pub fn decode<'a, T: Deserialize<'a>>(&'a self) -> Result<T, LawError> {
		serde_json::from_str(self.utf()?).map_err(|e| LawError::DeJson(e))
	}
}

pub struct LawsClient {
	client: Client<HttpsConnector<HttpConnector>>,
	nicknames: HashMap<String, String>,
	channel_names: HashMap<String, String>,
}
impl LawsClient {
	pub const API: &'static str = "https://discord.com/api/v9";
	pub const GUILD_ID: &'static str = "907657508292792342";
	pub fn new() -> Self {
		let https = HttpsConnector::new();
		Self {
			client: Client::builder().build::<_, hyper::Body>(https),
			nicknames: HashMap::new(),
			channel_names: HashMap::new(),
		}
	}
	pub async fn request(&self, uri: String) -> Result<Response, LawError> {
		debug!("Sending get to {}", &uri);
		let now = tokio::time::Instant::now();
		let mut req = Request::builder()
			.method(Method::GET)
			.uri(uri)
			.body(Body::from(""))
			.expect("request builder");

		req.headers_mut()
			.insert(AUTHORIZATION, HeaderValue::from_static(include_str!("token.txt")));

		let res = self.client.request(req).await.expect("Failed to request");

		// And then, if the request gets a response...
		let status = res.status();
		debug!("Recieved {} in {}ms", status, now.elapsed().as_millis());

		// Concatenate the body stream into a single buffer...

		let bytes = hyper::body::to_bytes(res).await.map_err(|e| LawError::Hyper(e))?;
		Ok(Response { bytes, status })
	}
	pub async fn get_nickname(&mut self, user_id: &String) -> Result<&String, LawError> {
		if !self.nicknames.contains_key(user_id) {
			let member = self
				.request(format!("{}/guilds/{}/members/{}", Self::API, Self::GUILD_ID, user_id))
				.await?
				.decode::<GuildMember>();

			self.nicknames.insert(
				user_id.clone(),
				if let Ok(member) = member {
					if let Some(n) = member.nick {
						n
					} else {
						member.user.username
					}
				} else {
					"Deleted User".to_string()
				},
			);
		}

		Ok(&self.nicknames.get(user_id).unwrap())
	}
	pub async fn get_channel_name(&mut self, channel_id: &String) -> Result<&String, LawError> {
		if !self.channel_names.contains_key(channel_id) {
			let channel = self.request(format!("{}/channels/{}", Self::API, channel_id)).await?.decode::<Channel>();

			self.channel_names.insert(
				channel_id.clone(),
				if let Ok(channel) = channel {
					channel.name
				} else {
					info!("Deleted channel {:?} {}", channel, channel_id);
					"Deleted Channel".to_string()
				},
			);
		}

		Ok(&self.channel_names.get(channel_id).unwrap())
	}
}
