use chrono::NaiveDateTime;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SelectableHelper, SqliteConnection};

use crate::schema;

#[derive(diesel::Queryable, diesel::Selectable, Debug)]
#[diesel(table_name = schema::tasks)]
pub struct TaskRow {
    pub task_id: String,
    pub payload_json: String,
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