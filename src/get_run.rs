use graphql_client::{GraphQLQuery, Response};
use reqwest::header::USER_AGENT;
use std::error::Error;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "queries/schema.json",
    query_path = "queries/get_run.graphql",
    response_derives = "Debug,Clone"
)]
pub struct RunQuery;

pub async fn get_run(
    run_id: String,
    dagster_uri: String,
) -> Result<run_query::ResponseData, Box<dyn Error + Send + Sync>> {
    let query_variables = run_query::Variables { run_id };
    let request_body = RunQuery::build_query(query_variables);

    let client = reqwest::Client::new();
    let res = client
        .post(dagster_uri)
        .header(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.36 Edge/16.16299")
        .json(&request_body)
        .send()
        .await?;

    let status = res.status();
    let body = res.text().await?;
    log::debug!("Run query status: {}", status);

    let response_body: Response<run_query::ResponseData> = serde_json::from_str(&body)?;
    log::debug!("Run query response: {:#?}", response_body);

    if let Some(data) = response_body.data {
        Ok(data)
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to fetch run data",
        )))
    }
}
