use color_eyre::eyre;

mod exploit;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    exploit::exploit1::attack_example(5).await;

    Ok(())
}
