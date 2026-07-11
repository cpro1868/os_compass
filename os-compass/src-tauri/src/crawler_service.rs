use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use std::net::TcpStream;
use std::time::Instant;
use crate::settings;
use crate::PROJECT_DIR;

pub struct CrawlerService {
    process: Option<Child>,
    port: u16,
    #[allow(dead_code)]
    last_check: Instant,
    was_running: bool,
}

impl CrawlerService {
    pub fn new() -> Self {
        Self {
            process: None,
            port: 8080,
            last_check: Instant::now(),
            was_running: false,
        }
    }

    pub fn is_running(&mut self) -> bool {
        let url = self.get_url();
        if TcpStream::connect_timeout(&url[7..].parse().ok().unwrap_or_else(|| "127.0.0.1:8080".parse().unwrap()), Duration::from_millis(100)).is_ok() {
            self.was_running = true;
            true
        } else {
            self.was_running = false;
            false
        }
    }

    pub fn start(&mut self) -> Result<String, String> {
        if self.is_running() {
            return Ok(self.get_url());
        }

        let settings = crate::settings::get_settings();

        let proxy = if settings.proxy_enabled && !settings.proxy_host.is_empty() {
            let proxy_url = if settings.proxy_username.is_empty() {
                format!("http://{}:{}", settings.proxy_host, settings.proxy_port)
            } else {
                format!("http://{}:{}@{}:{}",
                    settings.proxy_username,
                    settings.proxy_password,
                    settings.proxy_host,
                    settings.proxy_port
                )
            };
            format!("--proxy {}", proxy_url)
        } else {
            String::new()
        };

        let web2md_path = {
            let project_dir = PROJECT_DIR.lock().unwrap();
            let project_path = project_dir.as_ref()
                .map(|p| p.join("crawler").join("webtomd").join("web2md.exe"))
                .unwrap_or_default();
            
            if project_path.exists() {
                project_path.to_string_lossy().to_string()
            } else {
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.join("web2md.exe").to_string_lossy().to_string()))
                    .unwrap_or_else(|| "web2md.exe".to_string())
            }
        };

        let port = if !settings.crawler_api_url.is_empty() {
            settings.crawler_api_url.split(':').nth(2)
                .or_else(|| settings.crawler_api_url.split(':').last())
                .and_then(|p| p.trim_start_matches('/').parse().ok())
                .unwrap_or(8080)
        } else {
            8080
        };
        self.port = port;

        let args: Vec<String> = vec!["serve".to_string(), "--port".to_string(), port.to_string()];
        let proxy_args: Vec<String> = if proxy.is_empty() {
            vec![]
        } else {
            proxy.split_whitespace().map(|s| s.to_string()).collect()
        };
        let mut all_args = args;
        all_args.extend(proxy_args);
        
        match Command::new(&web2md_path)
            .args(&all_args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => {
                self.process = Some(child);
                std::thread::sleep(Duration::from_millis(500));
                Ok(self.get_url())
            }
            Err(e) => Err(format!("Failed to start web2md: {}", e)),
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub fn get_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

impl Drop for CrawlerService {
    fn drop(&mut self) {
        self.stop();
    }
}

lazy_static::lazy_static! {
    pub static ref CRAWLER_SERVICE: Mutex<CrawlerService> = Mutex::new(CrawlerService::new());
}

pub fn start_crawler_service() -> Result<String, String> {
    let mut service = CRAWLER_SERVICE.lock().map_err(|e| e.to_string())?;
    service.start()
}

pub fn stop_crawler_service() {
    let mut service = CRAWLER_SERVICE.lock().unwrap();
    service.stop();
}

pub fn get_crawler_url() -> Option<String> {
    let mut service = CRAWLER_SERVICE.lock().ok()?;
    if service.is_running() {
        Some(service.get_url())
    } else {
        None
    }
}