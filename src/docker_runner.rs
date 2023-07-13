use bollard::{
    container::{Config, CreateContainerOptions, LogOutput, LogsOptions},
    image::BuildImageOptions,
    Docker,
};
use futures::{StreamExt, TryStreamExt};
use rand::RngCore;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DockerError {
    #[error("bollard error")]
    BollardError(#[from] bollard::errors::Error),
}

pub struct DockerExploit {
    docker: Docker,
    image: String,
}

impl DockerExploit {
    pub async fn spawn(
        &self,
        host: String,
        flagid: String,
    ) -> Result<DockerExploitInstance, DockerError> {
        let name = format!(
            "instance_{image}_{host}",
            image = self.image,
            host = host.replace(".", "-"),
        );

        let config = Config {
            image: Some(self.image.clone()),
            tty: Some(true),
            env: Some(vec![format!("IP={host}"), format!("FLAG_ID={flagid}")]),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: name.clone(),
            ..Default::default()
        };

        let container = self.docker.create_container(Some(options), config).await?;

        self.docker
            .start_container::<String>(&container.id, None)
            .await?;

        Ok(DockerExploitInstance {
            container_id: container.id,
            //stream: todo!(),
            docker: self.docker.clone(),
        })
    }
}

pub struct DockerExploitInstance {
    docker: Docker,
    //stream: Box<dyn Stream<Item = Result<ContainerWaitResponse, bollard::errors::Error>>>,
    container_id: String,
}

impl DockerExploitInstance {
    pub async fn wait_for_join(&self) -> Result<Vec<LogOutput>, DockerError> {
        let mut waits = self.docker.wait_container::<&str>(&self.container_id, None);
        // shouldnt print anything
        while let Some(msg) = waits.next().await {
            let msg = msg?;
            eprintln!("Message: {:?}", msg);
        }

        let log_options = LogsOptions {
            stdout: true,
            stderr: true,
            follow: true, // ???
            ..Default::default()
        };

        let logs = self
            .docker
            .logs::<&str>(&self.container_id, Some(log_options))
            .try_collect::<Vec<_>>()
            .await?;

        Ok(logs)
    }
}

pub struct DockerInstance {
    docker: Docker,
}

impl DockerInstance {
    pub fn new() -> Result<Self, DockerError> {
        Ok(Self {
            docker: Docker::connect_with_local_defaults()?,
        })
    }

    pub async fn new_exploit(&self, tar: Vec<u8>) -> Result<DockerExploit, DockerError> {
        let image_name = {
            let mut bytes = [0u8; 8];
            rand::thread_rng().fill_bytes(&mut bytes);
            format!("exploit_{hash}", hash = hex::encode(bytes))
        };

        let options = BuildImageOptions {
            t: image_name.clone(),
            networkmode: String::from("host"),
            memory: Some(1024 * 1024 * 1024),
            ..Default::default()
        };

        let mut stream = self
            .docker
            .build_image(options, None, Some(tar.clone().into()));

        while let Some(msg) = stream.next().await {
            let msg = msg?;
            eprintln!("Message: {:?}", msg);
        }

        Ok(DockerExploit {
            image: image_name,
            docker: self.docker.clone(),
        })
    }
}

// how to get language server inside here with cfg(test)?
// should still work extactly the same, but may compile tests in normal builds...
//#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::DockerInstance;
    #[allow(unused_imports)]
    use color_eyre::eyre::Report;

    #[tokio::test]
    async fn simple_build() -> Result<(), Report> {
        color_eyre::install()?;

        let dockerfile = r###"
FROM ubuntu
CMD ["env"]
"###;
        // write the dockerfile to a tar file in memory
        let mut tar = tar::Builder::new(Vec::new());

        let mut header = tar::Header::new_gnu();
        header.set_size(dockerfile.len() as u64);
        header.set_cksum();
        tar.append_data(&mut header, "Dockerfile", dockerfile.as_bytes())?;
        let tar_data = tar.into_inner()?;

        // build it
        let docker = DockerInstance::new().unwrap();
        let exploit = docker.new_exploit(tar_data).await.unwrap();

        // run it
        let instance = exploit
            .spawn("172.0.1.2".to_string(), "flaghint".to_string())
            .await?;

        // get logs/output
        let logs = instance.wait_for_join().await?;
        for l in logs {
            eprint!("{}", l.to_string());
        }

        Ok(())
    }
}
