#!/usr/bin/env python3
"""
phi_helper.py - A utility to use Phi-3 for codebase assistance.

This tool helps with:
1. Code explanation
2. Documentation generation
3. Bug finding
4. Code suggestions

Requirements:
- openai package (pip install openai)
- A valid API key for OpenAI or Azure OpenAI with Phi-3 access
"""

import os
import sys
import argparse
import json
from pathlib import Path
import subprocess
from typing import Dict, List, Optional, Union, Any

try:
    import openai
except ImportError:
    print("Error: openai package not installed. Run 'pip install openai'")
    sys.exit(1)

CONFIG_PATH = Path.home() / ".config" / "phi_helper" / "config.json"

# Default configuration
DEFAULT_CONFIG = {
    "api_type": "openai",  # or "azure"
    "api_key": "",
    "azure_endpoint": "",  # Only needed for Azure
    "model": "phi-3",  # Default model
    "temperature": 0.3,
    "max_tokens": 4000,
    "repository_path": "",
}


def ensure_config():
    """Ensure config directory and file exist."""
    config_dir = CONFIG_PATH.parent
    if not config_dir.exists():
        config_dir.mkdir(parents=True)
    
    if not CONFIG_PATH.exists():
        with open(CONFIG_PATH, "w") as f:
            json.dump(DEFAULT_CONFIG, f, indent=2)
        print(f"Created default config at {CONFIG_PATH}")
        print(f"Please edit {CONFIG_PATH} and add your API key")
        sys.exit(1)

    return load_config()


def load_config():
    """Load configuration from file."""
    with open(CONFIG_PATH, "r") as f:
        config = json.load(f)
    
    # Validate required fields
    if config.get("api_key", "") == "":
        print(f"Error: API key not set in {CONFIG_PATH}")
        sys.exit(1)
    
    return config


def setup_client(config: Dict[str, Any]):
    """Set up OpenAI or Azure client."""
    if config["api_type"] == "azure":
        if not config.get("azure_endpoint"):
            print("Error: Azure endpoint not specified in config")
            sys.exit(1)
        
        client = openai.AzureOpenAI(
            api_key=config["api_key"],
            azure_endpoint=config["azure_endpoint"],
            api_version="2023-12-01-preview"
        )
    else:  # OpenAI
        client = openai.OpenAI(
            api_key=config["api_key"],
        )
    
    return client


def git_get_repo_files(repo_path: str, exclude_patterns: List[str] = None) -> List[str]:
    """Get list of all files tracked by git in the repository."""
    if exclude_patterns is None:
        exclude_patterns = [
            "target/", ".git/", "node_modules/", 
            ".DS_Store", "Cargo.lock", "*.lock", "*.log", 
            "gm_ml/lib/", "gm_ml/lib64/", "gm_ml/include/",
            "traces/", "trained-agents/"
        ]
    
    try:
        os.chdir(repo_path)
        result = subprocess.run(
            ["git", "ls-files"], 
            capture_output=True, 
            text=True, 
            check=True
        )
        all_files = result.stdout.splitlines()
        
        # Filter out excluded patterns
        filtered_files = []
        for file in all_files:
            if not any(pattern in file for pattern in exclude_patterns if pattern.endswith('/')):
                if not any(file.endswith(pattern.strip('*')) for pattern in exclude_patterns if pattern.startswith('*')):
                    filtered_files.append(file)
        
        return filtered_files
    except subprocess.CalledProcessError as e:
        print(f"Error running git ls-files: {e}")
        return []


def read_file_content(file_path: str, max_lines: Optional[int] = None) -> str:
    """Read content from a file with optional line limit."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            if max_lines:
                return ''.join([next(f) for _ in range(max_lines)])
            else:
                return f.read()
    except Exception as e:
        return f"Error reading file {file_path}: {e}"


def analyze_code(client, config: Dict[str, Any], files: List[str], query: str):
    """Analyze code files based on query."""
    file_contents = {}
    total_chars = 0
    char_limit = 100000  # Limit to ~100k characters to avoid token limits
    
    for file_path in files:
        if total_chars > char_limit:
            break
            
        if os.path.exists(file_path) and not os.path.isdir(file_path):
            content = read_file_content(file_path)
            file_size = len(content)
            
            if total_chars + file_size <= char_limit:
                file_contents[file_path] = content
                total_chars += file_size
            else:
                # Take as much as we can
                remaining = char_limit - total_chars
                if remaining > 1000:  # Only include if we can get a meaningful chunk
                    truncated = read_file_content(file_path, remaining // 80)  # Estimate ~80 chars per line
                    file_contents[file_path] = truncated + "\n... [truncated]"
                    total_chars = char_limit
    
    # Prepare system message
    system_message = f"""
You are Phi-3, a helpful AI coding assistant working with a Rust codebase for WasmAgentTrainer project.
Analyze the provided code files and answer the query concisely and accurately.
The project involves WebAssembly agent training and execution in a Rust environment.

When providing code examples, use idiomatic Rust when applicable.
Keep explanations clear, focused, and technically precise.
"""

    # Prepare user message
    user_message = f"Query: {query}\n\nCode files:\n"
    for file_path, content in file_contents.items():
        user_message += f"\n--- {file_path} ---\n{content}\n"
    
    # Call OpenAI API
    try:
        if config["api_type"] == "azure":
            response = client.chat.completions.create(
                model=config["model"],
                messages=[
                    {"role": "system", "content": system_message},
                    {"role": "user", "content": user_message}
                ],
                temperature=config["temperature"],
                max_tokens=config["max_tokens"],
            )
        else:
            response = client.chat.completions.create(
                model=config["model"],
                messages=[
                    {"role": "system", "content": system_message},
                    {"role": "user", "content": user_message}
                ],
                temperature=config["temperature"],
                max_tokens=config["max_tokens"],
            )
        
        return response.choices[0].message.content
    except Exception as e:
        return f"Error calling API: {str(e)}"


def command_setup(args):
    """Setup configuration."""
    config = ensure_config()
    
    print(f"Current configuration at {CONFIG_PATH}:")
    print(json.dumps(config, indent=2))
    
    # Interactive setup
    config["api_type"] = input(f"API type (openai, azure) [{config['api_type']}]: ") or config["api_type"]
    config["api_key"] = input(f"API key [{config['api_key'][:4] + '...' if config['api_key'] else ''}]: ") or config["api_key"]
    
    if config["api_type"] == "azure":
        config["azure_endpoint"] = input(f"Azure endpoint [{config['azure_endpoint']}]: ") or config["azure_endpoint"]
    
    config["model"] = input(f"Model [{config['model']}]: ") or config["model"]
    
    try:
        config["temperature"] = float(input(f"Temperature [{config['temperature']}]: ") or config["temperature"])
    except ValueError:
        print("Invalid temperature value, keeping previous value")
    
    try:
        config["max_tokens"] = int(input(f"Max tokens [{config['max_tokens']}]: ") or config["max_tokens"])
    except ValueError:
        print("Invalid max_tokens value, keeping previous value")
    
    config["repository_path"] = input(f"Repository path [{config['repository_path']}]: ") or config["repository_path"]
    
    # Save config
    with open(CONFIG_PATH, "w") as f:
        json.dump(config, f, indent=2)
    
    print(f"Configuration saved to {CONFIG_PATH}")


def command_analyze(args):
    """Analyze code based on query."""
    config = ensure_config()
    client = setup_client(config)
    
    repo_path = args.repo_path or config["repository_path"] or os.getcwd()
    
    if args.files:
        files = args.files
    else:
        files = git_get_repo_files(repo_path)
        if args.filter:
            files = [f for f in files if args.filter in f]
    
    print(f"Analyzing {len(files)} files with query: {args.query}")
    result = analyze_code(client, config, files, args.query)
    print("\n--- RESULT ---\n")
    print(result)


def command_docs(args):
    """Generate documentation for specified files."""
    config = ensure_config()
    client = setup_client(config)
    
    repo_path = args.repo_path or config["repository_path"] or os.getcwd()
    
    if not args.files:
        print("Error: Please specify at least one file to document")
        sys.exit(1)
    
    query = "Generate comprehensive documentation for these files. Include:\n" + \
            "1. Overall purpose of each module/file\n" + \
            "2. Main functions and their behavior\n" + \
            "3. Data structures and their fields\n" + \
            "4. Any important patterns or design decisions\n" + \
            "Format as Markdown."
    
    print(f"Generating documentation for {len(args.files)} files")
    result = analyze_code(client, config, args.files, query)
    
    if args.output:
        with open(args.output, "w") as f:
            f.write(result)
        print(f"Documentation written to {args.output}")
    else:
        print("\n--- DOCUMENTATION ---\n")
        print(result)


def command_suggest(args):
    """Suggest improvements for code."""
    config = ensure_config()
    client = setup_client(config)
    
    if not args.files:
        print("Error: Please specify at least one file to analyze")
        sys.exit(1)
    
    query = "Suggest improvements for this code. Look for:\n" + \
            "1. Potential bugs or edge cases\n" + \
            "2. Performance optimizations\n" + \
            "3. Better Rust idioms or patterns\n" + \
            "4. Improved error handling\n" + \
            "Provide specific code examples where possible."
    
    print(f"Analyzing {len(args.files)} files for suggestions")
    result = analyze_code(client, config, args.files, query)
    print("\n--- SUGGESTIONS ---\n")
    print(result)


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(description="Phi-3 Code Helper")
    subparsers = parser.add_subparsers(dest="command", help="Command to run")
    
    # Setup command
    parser_setup = subparsers.add_parser("setup", help="Configure API settings")
    
    # Analyze command
    parser_analyze = subparsers.add_parser("analyze", help="Analyze code")
    parser_analyze.add_argument("query", help="Query or question about the code")
    parser_analyze.add_argument("--files", nargs="*", help="Specific files to analyze")
    parser_analyze.add_argument("--filter", help="Filter files containing this string")
    parser_analyze.add_argument("--repo-path", help="Repository path")
    
    # Docs command
    parser_docs = subparsers.add_parser("docs", help="Generate documentation")
    parser_docs.add_argument("--files", nargs="*", help="Files to document")
    parser_docs.add_argument("--output", help="Output file for documentation")
    parser_docs.add_argument("--repo-path", help="Repository path")
    
    # Suggest command
    parser_suggest = subparsers.add_parser("suggest", help="Suggest code improvements")
    parser_suggest.add_argument("--files", nargs="*", required=True, help="Files to analyze")
    
    args = parser.parse_args()
    
    if args.command == "setup":
        command_setup(args)
    elif args.command == "analyze":
        command_analyze(args)
    elif args.command == "docs":
        command_docs(args)
    elif args.command == "suggest":
        command_suggest(args)
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
