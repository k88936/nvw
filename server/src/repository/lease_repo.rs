use chrono::NaiveDateTime;
use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl,
    SelectableHelper, SqliteConnection,
};

use crate::schema;

#[derive(diesel::Queryable, diesel::Selectable, Debug)]
#[diesel(table_name = schema::task_leases)]
struct TaskLeaseAttemptRow {
    attempt: i32,
}

#[derive(diesel::Insertable)]
#[diesel(table_name = schema::task_leases)]
struct NewTaskLeaseRow<'a> {
    lease_id: &'a str,
    task_id: &'a str,
    worker_id: &'a str,
    leased_at: NaiveDateTime,
    lease_expires_at: NaiveDateTime,
    attempt: i32,
    active: bool,
}

pub fn latest_attempt_for_task(
    conn: &mut SqliteConnection,
    task_id: &str,
) -> Result<Option<i32>, diesel::result::Error> {
    use schema::task_leases::dsl as lease_dsl;
    lease_dsl::task_leases
        .filter(lease_dsl::task_id.eq(task_id))
        .order(lease_dsl::leased_at.desc())
        .select(TaskLeaseAttemptRow::as_select())
        .first(conn)
        .optional()
        .map(|row| row.map(|v| v.attempt))
}

pub fn insert_lease(
    conn: &mut SqliteConnection,
    lease_id: &str,
    task_id: &str,
    worker_id: &str,
    leased_at: NaiveDateTime,
    lease_expires_at: NaiveDateTime,
    attempt: i32,
) -> Result<(), diesel::result::Error> {
    use schema::task_leases::dsl as lease_dsl;
    diesel::insert_into(lease_dsl::task_leases)
        .values(NewTaskLeaseRow {
            lease_id,
            task_id,
            worker_id,
            leased_at,
            lease_expires_at,
            attempt,
            active: true,
        })
        .execute(conn)?;
    Ok(())
}

pub fn lease_by_ids(
    conn: &mut SqliteConnection,
    lease_id: &str,
    task_id: &str,
    worker_id: &str,
) -> Result<Option<(bool, NaiveDateTime)>, diesel::result::Error> {
    use schema::task_leases::dsl as lease_dsl;
    lease_dsl::task_leases
        .filter(lease_dsl::lease_id.eq(lease_id))
        .filter(lease_dsl::task_id.eq(task_id))
        .filter(lease_dsl::worker_id.eq(worker_id))
        .select((lease_dsl::active, lease_dsl::lease_expires_at))
        .first(conn)
        .optional()
}

pub fn deactivate_lease(
    conn: &mut SqliteConnection,
    lease_id: &str,
) -> Result<(), diesel::result::Error> {
    use schema::task_leases::dsl as lease_dsl;
    diesel::update(lease_dsl::task_leases.filter(lease_dsl::lease_id.eq(lease_id)))
        .set(lease_dsl::active.eq(false))
        .execute(conn)?;
    Ok(())
}

pub fn deactivate_expired_leases(
    conn: &mut SqliteConnection,
    now: NaiveDateTime,
) -> anyhow::Result<usize> {
    use schema::task_leases::dsl as lease_dsl;
    use schema::tasks::dsl as task_dsl;

    let expired_task_ids: Vec<String> = lease_dsl::task_leases
        .filter(lease_dsl::active.eq(true))
        .filter(lease_dsl::lease_expires_at.lt(now))
        .select(lease_dsl::task_id)
        .load(conn)?;

    if expired_task_ids.is_empty() {
        return Ok(0);
    }

    conn.transaction::<(), diesel::result::Error, _>(|conn| {
        diesel::update(
            lease_dsl::task_leases
                .filter(lease_dsl::active.eq(true))
                .filter(lease_dsl::lease_expires_at.lt(now)),
        )
        .set(lease_dsl::active.eq(false))
        .execute(conn)?;

        diesel::update(
            task_dsl::tasks
                .filter(task_dsl::task_id.eq_any(&expired_task_ids))
                .filter(task_dsl::status.eq("running")),
        )
        .set((task_dsl::status.eq("failed"), task_dsl::updated_at.eq(now)))
        .execute(conn)?;
        Ok(())
    })?;

    Ok(expired_task_ids.len())
}
