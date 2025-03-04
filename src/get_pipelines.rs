use graphql_client::{GraphQLQuery, Response};
use reqwest::header::USER_AGENT;
use std::error::Error;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "queries/schema.json",
    query_path = "queries/get_pipelines.graphql",
    response_derives = "Debug,Clone"
)]
pub struct PipelinesQuery;

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub name: String,
    pub is_asset_job: bool,
    pub repository_location: String,
    pub last_run_status: Option<String>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self {
            name: String::new(),
            is_asset_job: false,
            repository_location: String::new(),
            last_run_status: None,
        }
    }
}

pub async fn get_pipelines(
    dagster_uri: String,
) -> Result<Vec<Pipeline>, Box<dyn Error + Send + Sync>> {
    let request_body = PipelinesQuery::build_query(pipelines_query::Variables {});

    let client = reqwest::Client::new();
    let res = client
        .post(dagster_uri)
        .header(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.36 Edge/16.16299")
        .json(&request_body)
        .send()
        .await?;

    let status = res.status();
    let body = res.text().await?;
    log::debug!("Pipelines query status: {}", status);

    let response_body: Response<pipelines_query::ResponseData> = serde_json::from_str(&body)?;
    log::debug!("Pipelines query response: {:#?}", response_body);

    if let Some(data) = response_body.data {
        match data.repositories_or_error {
            pipelines_query::PipelinesQueryRepositoriesOrError::RepositoryConnection(repo_conn) => {
                let mut pipelines = Vec::new();

                for repo_node in repo_conn.nodes {
                    let repo_name = repo_node.name;

                    for pipeline in repo_node.pipelines {
                        // Skip asset jobs
                        if pipeline.is_asset_job {
                            continue;
                        }

                        // Get repository location name
                        let repo_location =
                            pipeline.repository.origin.repository_location_name.clone();

                        log::debug!("Pipeline: {:?}, Repository: {:?}", pipeline.name, repo_name);

                        // Handle runs as a Vec instead of an Option
                        let last_run_status = if !pipeline.runs.is_empty() {
                            Some(format!("{:?}", pipeline.runs[0].status))
                        } else {
                            None
                        };

                        pipelines.push(Pipeline {
                            name: pipeline.name,
                            is_asset_job: pipeline.is_asset_job,
                            repository_location: repo_location,
                            last_run_status,
                        });
                    }
                }

                Ok(pipelines)
            }
            _ => Ok(Vec::new()),
        }
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to fetch pipelines data",
        )))
    }
}
