use crate::file;
use tokio::{fs::read_to_string, io::AsyncWriteExt};

pub async fn get_model_name() -> Result<String, Box<dyn std::error::Error>> {
    let contents = match read_to_string(file::get_config_file_path()).await {
        Ok(text) => text,
        Err(e) => return Err(e.into()),
    };

    let model_config = contents
        .lines()
        .filter(|line| line.starts_with("MISTRAL_MODEL"))
        .map(|line| line.split('=').collect::<Vec<&str>>())
        .map(|line| line[1].to_string())
        .collect::<Vec<String>>();

    let model_name = model_config[0].clone();
    let stripped_model_name = model_name.replace("\"", "");
    Ok(stripped_model_name)
}

pub async fn save_model_name(model_name: String) -> Result<(), Box<dyn std::error::Error>> {
    let (_, mut file) = file::create_config_file().await;

    _ = file.flush().await; // this is for nouw our only config
    file.write_all(format!("MISTRAL_MODEL=\"{}\"", model_name).as_bytes())
        .await?;

    Ok(())
}
