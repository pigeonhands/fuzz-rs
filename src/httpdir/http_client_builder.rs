use reqwest;
use reqwest::{header, redirect};
use base64::write::EncoderWriter as Base64Encoder;
use std::io::Write;

pub struct HttpClientBuilder<'a> {
    builder: reqwest::ClientBuilder,
    header_map: reqwest::header::HeaderMap,
    redirect_policy: redirect::Policy,
    gzip: bool,
    user_agent: &'a str,
    timeout_ms: Option<i32>,
}


impl<'a> HttpClientBuilder<'a> {

    pub fn new() -> Self{
        HttpClientBuilder{
            builder: reqwest::Client::builder(),
            header_map: reqwest::header::HeaderMap::new(),
            redirect_policy: redirect::Policy::default(),
            gzip: false,
            user_agent: "",
            timeout_ms: None,
        }
    }

    pub fn build(self) -> reqwest::Result<reqwest::Client> {
        let mut b = self.builder
            .default_headers(self.header_map)
            .gzip(self.gzip)
            .redirect(self.redirect_policy)
            .user_agent(self.user_agent);

        if let Some(t) = self.timeout_ms {
            if t > 0{
                b = b.timeout(std::time::Duration::from_millis(t as u64))
            }
        }
        b.build()
    }

    pub fn basic_auth(&mut self, username: &str, password: Option<&str>)-> Result<(), header::InvalidHeaderValue>{
        self.header_map.append(
            header::AUTHORIZATION,
            Self::create_auth_header(username, password)?,
        );
        Ok(())
    }

    pub fn gzip(&mut self, enabled: bool) {
       self.gzip = enabled;
    }
    
    pub fn timeout_ms(&mut self, t: i32) {
        if t > 0{
            self.timeout_ms = Some(t)
        }else{
            self.timeout_ms = None
        }
    }

    pub fn user_agent(&mut self, agent: &'a str) {
        self.user_agent = agent;
    }

    pub fn redirect_policy_keep_on_domain(&mut self, domain: &reqwest::Url, max_redirects: usize){
        let url = domain.clone();
        self.redirect_policy = redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() > max_redirects {
                attempt.error("too many redirects")
            } else if attempt.url().host_str() != url.host_str() {
                attempt.stop() //Do not redirect off target
            } else {
                attempt.follow()
            }
        });
    }


    fn create_auth_header<U, P>(
        username: U,
        password: Option<P>,
    ) -> Result<header::HeaderValue, header::InvalidHeaderValue>
    where
        U: std::fmt::Display,
        P: std::fmt::Display,
    {
        let mut header_value = b"Basic ".to_vec();
        {
            let mut encoder = Base64Encoder::new(&mut header_value, base64::STANDARD);
            write!(encoder, "{}:", username).unwrap();
            if let Some(password) = password {
                write!(encoder, "{}", password).unwrap();
            }
        }
        header::HeaderValue::from_bytes(&header_value)
    }

}