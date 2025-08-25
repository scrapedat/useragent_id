use anyhow::Result;
use octocrab::Octocrab;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use std::process::Command;
use tokio::process::Command as TokioCommand;
use std::collections::HashMap;
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepoMetadata {
    pub name: String,
    pub url: String,
    pub clone_url: String,
    pub stars: u32,
    pub description: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeSnippet {
    pub file: String,
    pub functions: Vec<FunctionData>,
    pub tests: Vec<TestData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionData {
    pub name: String,
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestData {
    pub code: String,
    pub context: String,
}

pub struct GitHubScraper {
    client: Octocrab,
    dataset_path: PathBuf,
    raw_path: PathBuf,
    processed_path: PathBuf,
}

impl GitHubScraper {
    pub async fn new(github_token: String) -> Result<Self> {
        let client = Octocrab::builder()
            .personal_token(github_token)
            .build()?;

        let dataset_path = PathBuf::from("./dataset");
        let raw_path = dataset_path.join("raw");
        let processed_path = dataset_path.join("processed");

        // Create directories
        for path in &[&dataset_path, &raw_path, &processed_path] {
            fs::create_dir_all(path).await?;
        }

        Ok(Self {
            client,
            dataset_path,
            raw_path,
            processed_path,
        })
    }

    pub async fn scrape_web_automation_repos(&self) -> Result<Vec<RepoMetadata>> {
        let search_queries = [
            "chromiumoxide rust web automation",
            "thirtyfour selenium rust",
            "headless_chrome rust automation",
            "playwright rust browser",
            "puppeteer rust automation",
            "webdriver rust selenium",
            "browser automation rust crate",
            "web scraping rust async",
        ];

        let mut repo_map = HashMap::new();

        for query in &search_queries {
            let repos = self.client
                .search()
                .repositories(query)
                .language("rust")
                .stars(">10")
                .sort("stars")
                .order("desc")
                .per_page(50)
                .send()
                .await?;

            for item in repos.items {
                if !repo_map.contains_key(&item.full_name) {
                    repo_map.insert(item.full_name.clone(), RepoMetadata {
                        name: item.full_name,
                        url: item.html_url.to_string(),
                        clone_url: item.clone_url.unwrap_or_default(),
                        stars: item.stargazers_count,
                        description: item.description,
                        language: item.language,
                    });
                }
            }

            // Rate limiting pause
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        Ok(repo_map.into_values().collect())
    }

    pub async fn download_repo_content(&self, repo: &RepoMetadata) -> Result<bool> {
        let repo_dir = self.raw_path.join(repo.name.replace('/', "_"));
        
        // Clone or update repository
        let clone_result = if repo_dir.exists() {
            TokioCommand::new("git")
                .args(&["pull"])
                .current_dir(&repo_dir)
                .output()
                .await
        } else {
            TokioCommand::new("git")
                .args(&["clone", "--depth", "1", &repo.clone_url, &repo_dir.to_string_lossy()])
                .output()
                .await
        };

        if clone_result.is_err() {
            return Ok(false);
        }

        // Find and process Rust files
        let rust_files = self.find_rust_files(&repo_dir);
        let relevant_code = self.extract_relevant_code(&rust_files).await?;

        if relevant_code.is_empty() {
            return Ok(false);
        }

        // Save extracted code
        let output_file = self.raw_path.join(format!("{}.json", repo.name.replace('/', "_")));
        let output_data = serde_json::json!({
            "repo": repo.name,
            "stars": repo.stars,
            "description": repo.description,
            "files": relevant_code,
            "extracted_at": chrono::Utc::now().to_rfc3339()
        });

        fs::write(&output_file, serde_json::to_string_pretty(&output_data)?).await?;

        // Clean up cloned repo
        fs::remove_dir_all(&repo_dir).await?;

        Ok(true)
    }

    fn find_rust_files(&self, repo_dir: &Path) -> Vec<PathBuf> {
        WalkDir::new(repo_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
            .map(|e| e.path().to_path_buf())
            .collect()
    }

    async fn extract_relevant_code(&self, rust_files: &[PathBuf]) -> Result<Vec<CodeSnippet>> {
        use tokio::task;
        use futures::StreamExt;
        use std::sync::Arc;

        let automation_keywords = Arc::new([
            "chromiumoxide", "thirtyfour", "headless_chrome", "playwright",
            "webdriver", "selenium", "browser", "page", "element", "click",
            "navigate", "find_element", "wait_for", "screenshot", "execute_script",
            "cookies", "local_storage", "session_storage", "iframe", "alert"
        ]);

        // Create a stream of files to process in parallel
        let mut tasks = futures::stream::iter(
            rust_files.iter().cloned().map(|path| {
                let keywords = automation_keywords.clone();
                task::spawn(async move {
                    if let Ok(content) = fs::read_to_string(&path).await {
                        let has_automation_code = keywords.iter()
                            .any(|&keyword| content.to_lowercase().contains(&keyword.to_lowercase()));

                        if has_automation_code {
                            let mut analyzer = CodeAnalyzer::new()?;
                            let analyzed_functions = analyzer.analyze_code(&content)?;
                            
                            let functions = analyzed_functions.into_iter()
                                .filter(|f| is_automation_function(&f.code))
                                .map(|f| FunctionData {
                                    name: f.name,
                                    code: f.code,
                                })
                                .collect::<Vec<_>>();

                            if !functions.is_empty() {
                                return Ok(Some(CodeSnippet {
                                    file: path.to_string_lossy().into_owned(),
                                    functions,
                                    tests: vec![], // Tests will be handled separately
                                }));
                            }
                        }
                    }
                    Ok(None)
                })
            })
        )
        .buffer_unordered(num_cpus::get()); // Process files in parallel

        let mut code_snippets = Vec::new();

        for file_path in rust_files {
            if let Ok(content) = fs::read_to_string(file_path).await {
                let has_automation_code = automation_keywords.iter()
                    .any(|&keyword| content.to_lowercase().contains(&keyword.to_lowercase()));

                if has_automation_code {
                    let functions = function_regex.captures_iter(&content)
                        .filter(|cap| self.is_automation_function(&cap[0]))
                        .map(|cap| FunctionData {
                            name: cap[1].to_string(),
                            code: cap[0].to_string(),
                        })
                        .collect::<Vec<_>>();

                    let tests = test_regex.captures_iter(&content)
                        .filter(|cap| self.is_automation_function(&cap[0]))
                        .map(|cap| TestData {
                            code: cap[0].to_string(),
                            context: "unit_test".to_string(),
                        })
                        .collect::<Vec<_>>();

                    if !functions.is_empty() || !tests.is_empty() {
                        code_snippets.push(CodeSnippet {
                            file: file_path.to_string_lossy().into_owned(),
                            functions,
                            tests,
                        });
                    }
                }
            }
        }

        Ok(code_snippets)
    }

    fn is_automation_function(&self, code: &str) -> bool {
        let patterns = [
            Regex::new(r"(?i)browser|page|element").unwrap(),
            Regex::new(r"(?i)click|navigate|find|wait").unwrap(),
            Regex::new(r"(?i)chromiumoxide|thirtyfour|headless_chrome").unwrap(),
        ];

        patterns.iter().any(|pattern| pattern.is_match(code))
    }
}
