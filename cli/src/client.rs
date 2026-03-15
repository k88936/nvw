use anyhow::{Context, Result};
use proto::{CreateTaskRequest, ListTasksResponse, TaskDto, TaskPayload, UpdateTaskRequest, TaskStatus, TaskResultDto};
use reqwest::{Client, Url};
use uuid::Uuid;

pub struct ApiClient {
    base_url: Url,
    client: Client,
    token: String,
}

impl ApiClient {
    pub fn new(base_url: &str, token: &str) -> Result<Self> {
        let url = Url::parse(base_url).context("Invalid base URL")?;
        Ok(Self {
            base_url: url,
            client: Client::new(),
            token: token.to_string(),
        })
    }

    fn url(&self, path: &str) -> Result<Url> {
        self.base_url.join(path).context("Invalid URL path")
    }

    async fn request_builder(&self, method: reqwest::Method, path: &str) -> Result<reqwest::RequestBuilder> {
        let url = self.url(path)?;
        Ok(self.client
            .request(method, url)
            .header("Authorization", format!("Bearer {}", self.token)))
    }

    pub async fn create_task(&self, payload: TaskPayload) -> Result<TaskDto> {
        let req = CreateTaskRequest { payload };
        let resp = self.request_builder(reqwest::Method::POST, "v1/tasks").await?
            .json(&req)
            .send()
            .await?;
            
        if !resp.status().is_success() {
            let error_text = resp.text().await?;
            anyhow::bail!("Failed to create task: {}", error_text);
        }

        resp.json().await.context("Failed to parse response")
    }

    pub async fn list_tasks(&self) -> Result<Vec<TaskDto>> {
        let resp = self.request_builder(reqwest::Method::GET, "v1/tasks").await?
            .send()
            .await?;

        if !resp.status().is_success() {
            let error_text = resp.text().await?;
            anyhow::bail!("Failed to list tasks: {}", error_text);
        }

        let wrapper: ListTasksResponse = resp.json().await.context("Failed to parse response")?;
        Ok(wrapper.tasks)
    }

    pub async fn get_task(&self, task_id: Uuid) -> Result<TaskDto> {
        let path = format!("v1/tasks/{}", task_id);
        let resp = self.request_builder(reqwest::Method::GET, &path).await?
            .send()
            .await?;

        if !resp.status().is_success() {
            let error_text = resp.text().await?;
            anyhow::bail!("Failed to get task: {}", error_text);
        }

        resp.json().await.context("Failed to parse response")
    }

    pub async fn get_task_result(&self, task_id: Uuid) -> Result<TaskResultDto> {
        let path = format!("v1/tasks/{}/result", task_id);
        let resp = self.request_builder(reqwest::Method::GET, &path).await?
            .send()
            .await?;

        if !resp.status().is_success() {
            let error_text = resp.text().await?;
            anyhow::bail!("Failed to get task result: {}", error_text);
        }

        resp.json().await.context("Failed to parse response")
    }

    pub async fn update_task(&self, task_id: Uuid, status: Option<TaskStatus>, payload: Option<TaskPayload>) -> Result<TaskDto> {
        let path = format!("v1/tasks/{}", task_id);
        let req = UpdateTaskRequest { status, payload };
        let resp = self.request_builder(reqwest::Method::PUT, &path).await?
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let error_text = resp.text().await?;
            anyhow::bail!("Failed to update task: {}", error_text);
        }

        resp.json().await.context("Failed to parse response")
    }

    pub async fn delete_task(&self, task_id: Uuid) -> Result<()> {
        let path = format!("v1/tasks/{}", task_id);
        let resp = self.request_builder(reqwest::Method::DELETE, &path).await?
            .send()
            .await?;

        if !resp.status().is_success() {
            let error_text = resp.text().await?;
            anyhow::bail!("Failed to delete task: {}", error_text);
        }

        Ok(())
    }
}
