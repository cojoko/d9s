use graphql_client::{GraphQLQuery, Response};
use reqwest::header::USER_AGENT;
use std::error::Error;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "queries/schema.json",
    query_path = "queries/get_runs.graphql",
    response_derives = "Debug,Clone"
)]
pub struct RunsQuery;

#[derive(Default)]
pub struct Variables {
    pub pipeline_name: String,
    pub cursor: String,
    pub run_ids: Vec<String>,
}

pub async fn get_runs(
    variables: Variables,
    dagster_uri: String,
    runs_limit: Option<usize>,
) -> Result<runs_query::ResponseData, Box<dyn Error + Send + Sync>> {
    let limit = runs_limit.unwrap_or(20);

    if !variables.pipeline_name.is_empty() {
        log::debug!("Filtering runs by pipeline: {}", variables.pipeline_name);
    }

    let query_variables = runs_query::Variables {
        pipeline_name: variables.pipeline_name,
        cursor: variables.cursor,
        run_ids: variables.run_ids,
        limit: limit as i64,
    };

    let request_body = RunsQuery::build_query(query_variables);
    log::debug!("Requesting runs from {dagster_uri}");

    let client = reqwest::Client::new();
    let res = client
        .post(dagster_uri)
        .header(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.36 Edge/16.16299")
        .json(&request_body)
        .send()
        .await?;

    let status = res.status();
    let body = res.text().await?;
    log::debug!("Runs query status: {}", status);

    let response_body: Response<runs_query::ResponseData> = serde_json::from_str(&body)?;
    log::debug!("Runs query response: {:#?}", response_body);

    if let Some(data) = response_body.data {
        Ok(data)
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to fetch runs data",
        )))
    }
}
