use anyhow::Result;
use fantoccini::{Client, ClientBuilder};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};
use tokio::time::sleep;
use webdriver::capabilities::Capabilities;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapingConfig {
    pub user_agents: Vec<String>,
    pub proxy_list: Vec<String>,
    pub delay_range: (u64, u64), // milliseconds
    pub max_retries: u32,
    pub viewport_sizes: Vec<(u32, u32)>,
    pub headers: HashMap<String, String>,
}

impl Default for ScrapingConfig {
    fn default() -> Self {
        Self {
            user_agents: vec![
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36".to_string(),
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36".to_string(),
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36".to_string(),
                // Add more modern user agents
            ],
            proxy_list: vec![], // To be filled from config
            delay_range: (1000, 3000),
            max_retries: 3,
            viewport_sizes: vec![
                (1920, 1080),
                (1366, 768),
                (1536, 864),
                (1440, 900),
            ],
            headers: HashMap::new(),
        }
    }
}

pub struct AdvancedScraper {
    config: ScrapingConfig,
    client: Option<Client>,
    rng: rand::rngs::ThreadRng,
}

impl AdvancedScraper {
    pub fn new(config: Option<ScrapingConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
            client: None,
            rng: rand::thread_rng(),
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        let mut caps = Capabilities::new();
        
        // Set random user agent
        let user_agent = self.get_random_user_agent();
        caps.insert("goog:chromeOptions".to_string(), serde_json::json!({
            "args": [
                "--no-sandbox",
                "--disable-dev-shm-usage",
                format!("--user-agent={}", user_agent),
                "--disable-blink-features=AutomationControlled", // Hide automation
                "--disable-infobars",
            ]
        }));

        // Set random viewport size
        let (width, height) = self.get_random_viewport();
        caps.insert("viewportSize".to_string(), serde_json::json!({
            "width": width,
            "height": height
        }));

        // Initialize client with capabilities
        self.client = Some(ClientBuilder::native()
            .capabilities(caps)
            .connect("http://localhost:4444")
            .await?);

        Ok(())
    }

    pub async fn navigate(&mut self, url: &str) -> Result<String> {
        let client = self.client.as_mut().expect("Client not initialized");
        
        // Random delay before navigation
        self.random_delay().await;

        // Navigate with retry logic
        let mut attempts = 0;
        while attempts < self.config.max_retries {
            match client.goto(url).await {
                Ok(_) => {
                    // Random scroll behavior
                    self.simulate_human_scrolling(client).await?;
                    
                    // Get page content
                    return Ok(client.source().await?);
                }
                Err(e) => {
                    attempts += 1;
                    if attempts == self.config.max_retries {
                        return Err(e.into());
                    }
                    // Exponential backoff
                    sleep(Duration::from_millis((1000 * attempts * attempts) as u64)).await;
                }
            }
        }

        Err(anyhow::anyhow!("Failed to navigate after retries"))
    }

    pub async fn click(&mut self, selector: &str) -> Result<()> {
        let client = self.client.as_mut().expect("Client not initialized");
        
        // Random delay before click
        self.random_delay().await;

        // Move mouse naturally to element
        self.simulate_human_mouse_movement(client, selector).await?;

        // Click with retry logic
        let mut attempts = 0;
        while attempts < self.config.max_retries {
            match client.find(fantoccini::Locator::Css(selector)).await?.click().await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    attempts += 1;
                    if attempts == self.config.max_retries {
                        return Err(e.into());
                    }
                    sleep(Duration::from_millis((1000 * attempts * attempts) as u64)).await;
                }
            }
        }

        Err(anyhow::anyhow!("Failed to click after retries"))
    }

    async fn simulate_human_scrolling(&mut self, client: &mut Client) -> Result<()> {
        // Get page height
        let height = client.execute(
            "return Math.max(document.documentElement.scrollHeight, document.body.scrollHeight);",
            vec![]
        ).await?.as_f64().unwrap_or(0.0) as i64;

        let mut current_pos = 0;
        while current_pos < height {
            // Random scroll amount
            let scroll_amount = self.rng.gen_range(100..300);
            current_pos += scroll_amount;

            // Smooth scroll with easing
            client.execute(
                &format!(
                    "window.scrollTo({{ top: {}, behavior: 'smooth' }});",
                    current_pos
                ),
                vec![]
            ).await?;

            // Random pause between scrolls
            sleep(Duration::from_millis(self.rng.gen_range(500..1500))).await;
        }

        Ok(())
    }

    async fn simulate_human_mouse_movement(&mut self, client: &mut Client, selector: &str) -> Result<()> {
        // Get element position
        let element = client.find(fantoccini::Locator::Css(selector)).await?;
        let rect = element.rect().await?;

        // Generate random bezier curve points for natural movement
        let (start_x, start_y) = (0.0, 0.0); // Current mouse position
        let (end_x, end_y) = (rect.x + (rect.width / 2.0), rect.y + (rect.height / 2.0));
        
        // Execute mouse movement along curve
        client.execute(
            &format!(
                r#"
                const bezierCurve = (t, p0, p1, p2, p3) => {{
                    const cX = 3 * (p1.x - p0.x),
                          bX = 3 * (p2.x - p1.x) - cX,
                          aX = p3.x - p0.x - cX - bX;
                    const cY = 3 * (p1.y - p0.y),
                          bY = 3 * (p2.y - p1.y) - cY,
                          aY = p3.y - p0.y - cY - bY;
                    const x = (aX * Math.pow(t, 3)) + (bX * Math.pow(t, 2)) + (cX * t) + p0.x;
                    const y = (aY * Math.pow(t, 3)) + (bY * Math.pow(t, 2)) + (cY * t) + p0.y;
                    return {{x, y}};
                }};
                const p0 = {{x: {}, y: {}}};
                const p3 = {{x: {}, y: {}}};
                const p1 = {{x: p0.x + {}, y: p0.y + {}}};
                const p2 = {{x: p3.x - {}, y: p3.y - {}}};
                for(let t = 0; t <= 1; t += 0.1) {{
                    const pos = bezierCurve(t, p0, p1, p2, p3);
                    // Would implement actual mouse movement here if we had lower level control
                    console.log(`Mouse moved to: ${{pos.x}}, ${{pos.y}}`);
                }}
                "#,
                start_x, start_y, end_x, end_y,
                self.rng.gen_range(50.0..150.0), self.rng.gen_range(20.0..100.0),
                self.rng.gen_range(50.0..150.0), self.rng.gen_range(20.0..100.0)
            ),
            vec![]
        ).await?;

        Ok(())
    }

    fn get_random_user_agent(&mut self) -> String {
        self.config.user_agents[self.rng.gen_range(0..self.config.user_agents.len())].clone()
    }

    fn get_random_viewport(&mut self) -> (u32, u32) {
        self.config.viewport_sizes[self.rng.gen_range(0..self.config.viewport_sizes.len())]
    }

    async fn random_delay(&mut self) {
        let delay = self.rng.gen_range(self.config.delay_range.0..self.config.delay_range.1);
        sleep(Duration::from_millis(delay)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scraper_initialization() {
        let mut scraper = AdvancedScraper::new(None);
        assert!(scraper.init().await.is_ok());
    }

    #[tokio::test]
    async fn test_navigation_with_retries() {
        let mut scraper = AdvancedScraper::new(None);
        scraper.init().await.unwrap();
        
        // Test with a known reliable site
        let result = scraper.navigate("https://example.com").await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Example Domain"));
    }
}
