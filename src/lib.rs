use serde_json::Value;

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

    pub fn find_release(self) -> Result<(Option<ReleaseManager>, Option<ReleaseManager>), minreq::Error> {
        let url = format!("https://api.github.com/repos/{}/{}/releases", self.author, self.repository);
        let request = minreq::Request::new(minreq::Method::Get, url)
            .with_header("Accept", "application/vnd.github.v3+json")
            .with_header("User-Agent", self.client.as_str());
        let request = if let Some(token) = self.auth_token.as_ref() {
            request.with_header("Authorization", format!("token {}", token).as_str())
        } else {
            request
        };
        let response = request.send()?;
        let json_response: Vec<Value> = match serde_json::from_str(response.as_str()?) {
            Ok(response) => response,
            Err(_) => return Err(minreq::Error::Other("Failed to parse GitHub JSON Respone!"))
        };

        let mut latest_release = None;
        let mut latest_prerelease = None;

        for release in json_response.into_iter() {
            if self.allow_prerelease && latest_prerelease.is_none() && release["prerelease"].as_bool().expect("GitHub Release JSON Invalid!") {
                let asset_url = release["assets_url"].as_str().expect("GitHub release assets_url is invalid!");
                let request = minreq::Request::new(minreq::Method::Get, asset_url)
                    .with_header("Accept", "application/vnd.github.v3+json")
                    .with_header("User-Agent", self.client.as_str());
                let request = if let Some(token) = self.auth_token.as_ref() {
                    request.with_header("Authorization", format!("token {}", token).as_str())
                } else {
                    request
                };
                let response = request.send()?;
                let json_response: Vec<Value> = match serde_json::from_str(response.as_str()?) {
                    Ok(response) => response,
                    Err(_) => return Err(minreq::Error::Other("Failed to parse GitHub assets JSON response!"))
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
                let request = minreq::Request::new(minreq::Method::Get, asset_url)
                    .with_header("Accept", "application/vnd.github.v3+json")
                    .with_header("User-Agent", self.client.as_str());
                let request = if let Some(token) = self.auth_token.as_ref() {
                    request.with_header("Authorization", format!("token {}", token).as_str())
                } else {
                    request
                };
                let response = request.send()?;
                let json_response: Vec<Value> = match serde_json::from_str(response.as_str()?) {
                    Ok(response) => response,
                    Err(_) => return Err(minreq::Error::Other("Failed to parse GitHub assets JSON response!"))
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
                    let request = minreq::Request::new(minreq::Method::Get, asset["url"].as_str().expect("GitHub Asset JSON Invalid!"))
                        .with_header("Accept", "application/octet-stream")
                        .with_header("User-Agent", self.client_name.as_str());
                    let request = if let Some(token) = self.auth_token.as_ref() {
                        request.with_header("Authorization", format!("token {}", token).as_str())
                    } else {
                        request
                    };
                    let response = match request.send() {
                        Ok(response) => response.into_bytes(),
                        Err(e) => {
                            println!("{:?}", e);
                            return None;
                        }
                    };
                    return Some(response)
                }
            }
        }
        None
    }

    pub fn get_asset_by_name_with_progress<S: AsRef<str>>(&self, name: S, on_download: impl Fn(usize, usize)) -> Option<Vec<u8>> {
        for asset in self.assets.iter() {
            if let Some(asset_name) = asset["name"].as_str() {
                if asset_name == name.as_ref() {
                    let request = minreq::Request::new(minreq::Method::Get, asset["url"].as_str().expect("GitHub Asset JSON Invalid!"))
                        .with_header("Accept", "application/octet-stream")
                        .with_header("User-Agent", self.client_name.as_str());
                    let request = if let Some(token) = self.auth_token.as_ref() {
                        request.with_header("Authorization", format!("token {}", token).as_str())
                    } else {
                        request
                    };
                    let response = match request.send_lazy() {
                        Ok(response) => response,
                        Err(e) => {
                            println!("{:?}", e);
                            return None;
                        }
                    };
                    let mut vec = Vec::new();
                    for (count, result) in response.into_iter().enumerate() {
                        let (byte, length) = match result {
                            Ok(res) => res,
                            Err(e) => {
                                println!("{:?}", e);
                                return None;
                            }
                        };
                        on_download(count, length);
                        vec.reserve(length);
                        vec.push(byte);
                    }
                    return Some(vec)
                }
            }
        }
        None
    }
}