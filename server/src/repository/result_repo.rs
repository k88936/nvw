use chrono::NaiveDateTime;
use diesel::{RunQueryDsl, SqliteConnection};

use crate::schema;

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
