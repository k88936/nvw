use chrono::NaiveDateTime;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SelectableHelper, SqliteConnection};

use crate::schema;
use diesel::prelude::*;
use proto::{TaskDto, TaskPayload, TaskStatus};
use std::str::FromStr;
use uuid::Uuid;

#[derive(diesel::Queryable, diesel::Selectable, Debug)]
#[diesel(table_name = schema::tasks)]
pub struct TaskRow {
    pub task_id: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub status: String,
    pub payload_json: String,
}

#[derive(Insertable)]
#[diesel(table_name = schema::tasks)]
pub struct NewTaskRow<'a> {
    pub task_id: &'a str,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub status: &'a str,
    pub payload_json: &'a str,
}

impl TaskRow {
    pub fn into_dto(self) -> anyhow::Result<TaskDto> {
        Ok(TaskDto {
            id: Uuid::from_str(&self.task_id)?,
            status: match self.status.as_str() {
                "pending" => TaskStatus::Pending,
                "running" => TaskStatus::Running,
                "succeeded" => TaskStatus::Succeeded,
                "failed" => TaskStatus::Failed,
                _ => TaskStatus::Pending, // Default or error?
            },
            payload: serde_json::from_str(&self.payload_json)?,
            created_at: chrono::DateTime::from_naive_utc_and_offset(self.created_at, chrono::Utc),
            updated_at: chrono::DateTime::from_naive_utc_and_offset(self.updated_at, chrono::Utc),
        })
    }
}

pub fn create_task(
    conn: &mut SqliteConnection,
    payload: &TaskPayload,
) -> anyhow::Result<TaskDto> {
    use schema::tasks::dsl as task_dsl;

    let now = chrono::Utc::now().naive_utc();
    let task_id = Uuid::new_v4().to_string();
    let payload_json = serde_json::to_string(payload)?;

    let new_task = NewTaskRow {
        task_id: &task_id,
        created_at: now,
        updated_at: now,
        status: "pending",
        payload_json: &payload_json,
    };

    diesel::insert_into(task_dsl::tasks)
        .values(&new_task)
        .execute(conn)?;

    Ok(TaskDto {
        id: Uuid::from_str(&task_id)?,
        status: TaskStatus::Pending,
        payload: payload.clone(),
        created_at: chrono::DateTime::from_naive_utc_and_offset(now, chrono::Utc),
        updated_at: chrono::DateTime::from_naive_utc_and_offset(now, chrono::Utc),
    })
}

pub fn get_task(conn: &mut SqliteConnection, task_id: &Uuid) -> anyhow::Result<Option<TaskDto>> {
    use schema::tasks::dsl as task_dsl;

    let task = task_dsl::tasks
        .filter(task_dsl::task_id.eq(task_id.to_string()))
        .select(TaskRow::as_select())
        .first(conn)
        .optional()?;

    match task {
        Some(t) => Ok(Some(t.into_dto()?)),
        None => Ok(None),
    }
}

pub fn list_tasks(conn: &mut SqliteConnection) -> anyhow::Result<Vec<TaskDto>> {
    use schema::tasks::dsl as task_dsl;

    let tasks = task_dsl::tasks
        .select(TaskRow::as_select())
        .load::<TaskRow>(conn)?;

    tasks.into_iter().map(|t| t.into_dto()).collect()
}

pub fn update_task(
    conn: &mut SqliteConnection,
    task_id: &Uuid,
    status: Option<TaskStatus>,
    payload: Option<TaskPayload>,
) -> anyhow::Result<Option<TaskDto>> {
    use schema::tasks::dsl as task_dsl;

    let now = chrono::Utc::now().naive_utc();
    let target = task_dsl::tasks.filter(task_dsl::task_id.eq(task_id.to_string()));

    if let Some(s) = status {
        let status_str = match s {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Succeeded => "succeeded",
            TaskStatus::Failed => "failed",
        };
        diesel::update(target.clone())
            .set((task_dsl::status.eq(status_str), task_dsl::updated_at.eq(now)))
            .execute(conn)?;
    }

    if let Some(p) = payload {
        let payload_json = serde_json::to_string(&p)?;
        diesel::update(target.clone())
            .set((
                task_dsl::payload_json.eq(payload_json),
                task_dsl::updated_at.eq(now),
            ))
            .execute(conn)?;
    }

    get_task(conn, task_id)
}

pub fn delete_task(conn: &mut SqliteConnection, task_id: &Uuid) -> anyhow::Result<bool> {
    use schema::tasks::dsl as task_dsl;

    let count = diesel::delete(task_dsl::tasks.filter(task_dsl::task_id.eq(task_id.to_string())))
        .execute(conn)?;
    
    Ok(count > 0)
}

pub fn fetch_first_pending_task(
    conn: &mut SqliteConnection,
) -> Result<Option<TaskRow>, diesel::result::Error> {
    use schema::tasks::dsl as task_dsl;

    task_dsl::tasks
        .filter(task_dsl::status.eq("pending"))
        .select(TaskRow::as_select())
        .first(conn)
        .optional()
}

pub fn mark_task_running(
    conn: &mut SqliteConnection,
    task_id: &str,
    now: NaiveDateTime,
) -> Result<(), diesel::result::Error> {
    use schema::tasks::dsl as task_dsl;
    diesel::update(task_dsl::tasks.filter(task_dsl::task_id.eq(task_id)))
        .set((task_dsl::status.eq("running"), task_dsl::updated_at.eq(now)))
        .execute(conn)?;
    Ok(())
}

pub fn mark_task_status(
    conn: &mut SqliteConnection,
    task_id: &str,
    status: &str,
    now: NaiveDateTime,
) -> Result<(), diesel::result::Error> {
    use schema::tasks::dsl as task_dsl;
    diesel::update(task_dsl::tasks.filter(task_dsl::task_id.eq(task_id)))
        .set((task_dsl::status.eq(status), task_dsl::updated_at.eq(now)))
        .execute(conn)?;
    Ok(())
}

pub fn requeue_failed_task(conn: &mut SqliteConnection) -> anyhow::Result<()> {
    use diesel::Connection;
    use crate::schema::tasks::dsl as task_dsl;
    conn.transaction(|conn| {
        diesel::update(task_dsl::tasks.filter(task_dsl::status.eq("failed")))
            .set(task_dsl::status.eq("pending"))
            .execute(conn)
    })?;
    Ok(())
}