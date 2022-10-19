use serde_json::Value;
use smashnet::types::*;

pub struct ReleaseFinderConfig {
    auth_token: Option<String>,
    client: String,
    allow_prerelease: bool,
    author: String,
    repository: String
}

impl ReleaseFinderConfig {
    pub fn new<S: AsRef<str>>(client_name: S) -> Self {
        Self {
            auth_token: None,
            client: client_name.as_ref().to_string(),
            allow_prerelease: false,
            author: String::new(),
            repository: String::new()
        }
    }

    pub fn with_token<S: AsRef<str>>(mut self, token: Option<S>) -> Self {
        let token = if let Some(token) = token {
            Some(token.as_ref().to_string())
        } else {
            None
        };
        self.auth_token = token;
        self
    }

    pub fn with_prereleases(mut self, allow_prerelease: bool) -> Self {
        self.allow_prerelease = allow_prerelease;
        self
    }

    pub fn with_author<S: AsRef<str>>(mut self, author: S) -> Self {
        self.author = author.as_ref().to_string();
        self
    }

    pub fn with_repository<S: AsRef<str>>(mut self, repository: S) -> Self {
        self.repository = repository.as_ref().to_string();
        self
    }

    pub fn find_release(self) -> Result<(Option<ReleaseManager>, Option<ReleaseManager>), String> {
        let url = format!("https://api.github.com/repos/{}/{}/releases", self.author, self.repository);
        let mut request: Curler = Curler::new();
        let response = request.get(url).unwrap();
        let json_response: Vec<Value> = match serde_json::from_str(response.as_str()) {
            Ok(response) => response,
            Err(_) => return Err("Failed to parse GitHub JSON Respone!".to_string())
        };

        let mut latest_release = None;
        let mut latest_prerelease = None;

        for release in json_response.into_iter() {
            if self.allow_prerelease && latest_prerelease.is_none() && release["prerelease"].as_bool().expect("GitHub Release JSON Invalid!") {
                let asset_url = release["assets_url"].as_str().expect("GitHub release assets_url is invalid!");
                let mut request: Curler = Curler::new();
                let response = request.get(asset_url.to_string()).unwrap();
                let json_response: Vec<Value> = match serde_json::from_str(response.as_str()) {
                    Ok(response) => response,
                    Err(_) => return Err("Failed to parse GitHub assets JSON response!".to_string())
                };
                latest_prerelease = Some(ReleaseManager {
                    client_name: self.client.clone(),
                    auth_token: self.auth_token.clone(),
                    version_tag: release["tag_name"].as_str().expect("Failed to parse version tag from GitHub release JSON!").to_string(),
                    assets: json_response
                });
            }
            
            if latest_release.is_none() && !release["prerelease"].as_bool().expect("GitHub Release JSON Invalid!") {
                let asset_url = release["assets_url"].as_str().expect("GitHub release assets_url is invalid!");
                let mut request: Curler = Curler::new();
                let response = request.get(asset_url.to_string()).unwrap();
                let json_response: Vec<Value> = match serde_json::from_str(response.as_str()) {
                    Ok(response) => response,
                    Err(_) => return Err("Failed to parse GitHub assets JSON response!".to_string())
                };
                latest_release = Some(ReleaseManager {
                    client_name: self.client.clone(),
                    auth_token: self.auth_token.clone(),
                    version_tag: release["tag_name"].as_str().expect("Failed to parse version tag from GitHub release JSON!").to_string(),
                    assets: json_response
                });
            }

            if latest_release.is_some() && (latest_prerelease.is_some() || !self.allow_prerelease) { 
                break; 
            }
        }

        Ok((latest_release, latest_prerelease))
    }
}

pub struct ReleaseManager {
    client_name: String,
    auth_token: Option<String>,
    version_tag: String,
    assets: Vec<Value>
}

impl ReleaseManager {
    pub fn get_release_tag(&self) -> &str {
        self.version_tag.as_str()
    }

    pub fn get_asset_names(&self) -> Vec<&str> {
        self.assets.iter().filter_map(|x| x["name"].as_str()).collect()
    }

    pub fn get_asset_by_name<S: AsRef<str>>(&self, name: S) -> Option<Vec<u8>> {
        for asset in self.assets.iter() {
            if let Some(asset_name) = asset["name"].as_str() {
                if asset_name == name.as_ref() {
                    let mut request: Curler = Curler::new();
                    let mut buffer = Vec::new();
                    let response = match request.get_bytes(asset["url"].to_string(), &mut buffer){
                        Ok(response) => {
                            dbg!(response.clone());
                            &buffer
                        },
                        Err(e) => {
                            println!("{:?}", e);
                            return None;
                        }
                    };
                    return Some(buffer)
                }
            }
        }
        None
    }
}