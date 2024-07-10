use log::info;

pub fn pull(image: &String) -> anyhow::Result<()> {
    info!("Attempting to pull image, with name {}", image);
    Ok(())
}
