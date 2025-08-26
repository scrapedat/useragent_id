use anyhow::Result;

mod dataset_builder;

fn main() -> Result<()> {
    let out = "data/internal.jsonl";
    dataset_builder::build_internal_dataset("src", out)?;
    println!("Dataset written to {}", out);
    Ok(())
}
