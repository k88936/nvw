use chrono::{NaiveDateTime, Utc};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SelectableHelper, SqliteConnection};
use proto::{FailedOptimization, ResultOutcome, SuccessfulOptimization, TaskResultDto, TaskRunMetrics};
use uuid::Uuid;
use std::str::FromStr;

use crate::schema;

#[derive(diesel::Queryable, diesel::Selectable)]
#[diesel(table_name = schema::task_results)]
pub struct TaskResultRow {
    pub lease_id: String,
    pub task_id: String,
    pub worker_id: String,
    pub outcome: String,
    pub best_cost: Option<f32>,
    pub best_param_json: Option<String>,
    pub iters: i32,
    pub best_iters: i32,
    pub termination: String,
    pub error_message: Option<String>,
    pub finished_at: NaiveDateTime,
}

#[derive(diesel::Insertable)]
#[diesel(table_name = schema::task_results)]
struct NewTaskResultRow<'a> {
    lease_id: &'a str,
    task_id: &'a str,
    worker_id: &'a str,
    outcome: &'a str,
    best_cost: Option<f32>,
    best_param_json: Option<&'a str>,
    iters: i32,
    best_iters: i32,
    termination: &'a str,
    error_message: Option<&'a str>,
    finished_at: NaiveDateTime,
}

pub struct NewTaskResult<'a> {
    pub lease_id: &'a str,
    pub task_id: &'a str,
    pub worker_id: &'a str,
    pub outcome: &'a str,
    pub best_cost: Option<f32>,
    pub best_param_json: Option<&'a str>,
    pub iters: i32,
    pub best_iters: i32,
    pub termination: &'a str,
    pub error_message: Option<&'a str>,
    pub finished_at: NaiveDateTime,
}

pub fn insert_task_result(
    conn: &mut SqliteConnection,
    input: NewTaskResult<'_>,
) -> Result<(), diesel::result::Error> {
    use schema::task_results::dsl as result_dsl;
    diesel::insert_into(result_dsl::task_results)
        .values(NewTaskResultRow {
            lease_id: input.lease_id,
            task_id: input.task_id,
            worker_id: input.worker_id,
            outcome: input.outcome,
            best_cost: input.best_cost,
            best_param_json: input.best_param_json,
            iters: input.iters,
            best_iters: input.best_iters,
            termination: input.termination,
            error_message: input.error_message,
            finished_at: input.finished_at,
        })
        .execute(conn)?;
    Ok(())
}

pub fn get_task_result(
    conn: &mut SqliteConnection,
    task_id: &str,
) -> anyhow::Result<Option<TaskResultDto>> {
    use schema::task_results::dsl as result_dsl;

    let result = result_dsl::task_results
        .filter(result_dsl::task_id.eq(task_id))
        .select(TaskResultRow::as_select())
        .first(conn)
        .optional()?;

    let Some(row) = result else {
        return Ok(None);
    };

    let outcome = match row.outcome.as_str() {
        "succeeded" => ResultOutcome::Succeeded,
        "failed" => ResultOutcome::Failed,
        _ => anyhow::bail!("Invalid outcome in database: {}", row.outcome),
    };

    let success = if outcome == ResultOutcome::Succeeded {
        Some(SuccessfulOptimization {
            best_cost: row.best_cost.unwrap_or_default(),
            best_param: row
                .best_param_json
                .map(|s| serde_json::from_str(&s))
                .transpose()?
                .unwrap_or_default(),
        })
    } else {
        None
    };

    let failure = if outcome == ResultOutcome::Failed {
        Some(FailedOptimization {
            error_message: row.error_message.unwrap_or_default(),
        })
    } else {
        None
    };

    Ok(Some(TaskResultDto {
        task_id: Uuid::from_str(&row.task_id)?,
        lease_id: Uuid::from_str(&row.lease_id)?,
        worker_id: Uuid::from_str(&row.worker_id)?,
        outcome,
        metrics: TaskRunMetrics {
            iters: row.iters as usize,
            best_iters: row.best_iters as usize,
            termination: row.termination,
        },
        success,
        failure,
        finished_at: chrono::DateTime::from_naive_utc_and_offset(row.finished_at, Utc),
    }))
}
