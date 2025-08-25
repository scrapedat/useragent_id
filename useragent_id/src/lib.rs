pub mod api;
pub mod core;
pub mod training;
pub mod wasm;

// Re-export commonly used types
pub use crate::core::types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::training::recorder::TaskRecorder;
    use crate::training::scraper::GitHubScraper;
    use crate::wasm::orchestrator::WasmOrchestrator;

    #[tokio::test]
    async fn test_github_scraper() {
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            let scraper = GitHubScraper::new(&token).await.expect("Failed to create scraper");
            scraper.initialize().await.expect("Failed to initialize scraper");
            
            let repos = scraper.scrape_web_automation_repos().await.expect("Failed to scrape repos");
            assert!(!repos.is_empty(), "Should find some repositories");

            // Test downloading first repo
            if let Some(repo) = repos.first() {
                let success = scraper.download_repo_content(repo).await.expect("Failed to download repo");
                assert!(success, "Should successfully download and process repo");
            }
        }
    }

    #[tokio::test]
    async fn test_basic_training_pipeline() {
        // 1. Create a recorder
        let mut recorder = TaskRecorder::new();

        // 2. Record some events
        recorder.record_event(DOMEvent {
            event_type: "click".to_string(),
            element_tag: "button".to_string(),
            xpath: "/html/body/div/button[1]".to_string(),
        });

        recorder.record_voice(VoiceAnnotation {
            text: "Click the submit button".to_string(),
            confidence: 0.95,
        });

        // 3. Generate a training plan
        let plan = recorder.generate_training_plan();
        assert!(!plan.steps.is_empty(), "Training plan should have steps");

        // 4. Create orchestrator and execute plan
        let orchestrator = WasmOrchestrator::new().expect("Should create orchestrator");
        let result = orchestrator.execute_training_plan(plan).await;
        assert!(result.is_ok(), "Should execute training plan without errors");
    }

    #[tokio::test]
    async fn test_github_scraper() {
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            let scraper = GitHubScraper::new(token).await.expect("Should create scraper");
            
            // Test repository search
            let repos = scraper.scrape_web_automation_repos().await.expect("Should find repos");
            assert!(!repos.is_empty(), "Should find some repositories");
            
            // Verify repo metadata
            for repo in &repos {
                assert!(!repo.name.is_empty(), "Repository name should not be empty");
                assert!(repo.stars > 0, "Repository should have stars");
                assert!(repo.clone_url.contains("github.com"), "Should be a GitHub URL");
            }
            
            // Test parallel repo processing
            let mut successes = 0;
            let mut futures = Vec::new();
            
            for repo in repos.iter().take(3) {
                let scraper = scraper.clone();
                let repo = repo.clone();
                futures.push(tokio::spawn(async move {
                    scraper.download_repo_content(&repo).await
                }));
            }
            
            for future in futures {
                if let Ok(Ok(success)) = future.await {
                    if success {
                        successes += 1;
                    }
                }
            }
            
            assert!(successes > 0, "Should successfully process at least one repository");
        }
    }
    
    #[tokio::test]
    async fn test_code_analyzer() {
        let mut analyzer = CodeAnalyzer::new().expect("Should create analyzer");
        
        let test_code = r#"
        /// This is a test function
        pub async fn test_automation(url: &str) -> Result<(), Error> {
            let browser = Browser::new().await?;
            let page = browser.new_page(url).await?;
            
            page.click("button").await?;
            page.type_text("input", "test").await?;
            
            Ok(())
        }
        "#;
        
        let analyzed = analyzer.analyze_code(test_code).expect("Should analyze code");
        assert_eq!(analyzed.len(), 1, "Should find one function");
        
        let func = &analyzed[0];
        assert_eq!(func.name, "test_automation");
        assert!(func.async_status, "Should be async");
        assert_eq!(func.visibility, "pub");
        assert!(func.documentation.is_some(), "Should have documentation");
        assert_eq!(func.parameters.len(), 1, "Should have one parameter");
        assert!(func.complexity > 1, "Should have complexity > 1");
    }
    
    #[tokio::test]
    async fn test_integrated_pipeline() {
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            let scraper = GitHubScraper::new(token).await.expect("Should create scraper");
            let mut recorder = TaskRecorder::new();
            let orchestrator = WasmOrchestrator::new().expect("Should create orchestrator");
            
            // 1. Scrape repositories
            let repos = scraper.scrape_web_automation_repos().await.expect("Should find repos");
            assert!(!repos.is_empty(), "Should find repositories");
            
            // 2. Record example automation
            recorder.record_event(DOMEvent {
                event_type: "click".to_string(),
                element_tag: "button".to_string(),
                xpath: "/html/body/div/button[1]".to_string(),
            });
            
            // 3. Generate and execute training plan
            let plan = recorder.generate_training_plan();
            let result = orchestrator.execute_training_plan(plan).await;
            assert!(result.is_ok(), "Should execute training plan");
        }
    }
}
