diesel::table! {
    tasks (task_id) {
        task_id -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        status -> Text,
        payload_json -> Text,
    }
}

diesel::table! {
    task_leases (lease_id) {
        lease_id -> Text,
        task_id -> Text,
        worker_id -> Text,
        leased_at -> Timestamp,
        lease_expires_at -> Timestamp,
        attempt -> Integer,
        active -> Bool,
    }
}

diesel::table! {
    task_results (lease_id) {
        lease_id -> Text,
        task_id -> Text,
        worker_id -> Text,
        outcome -> Text,
        best_cost -> Nullable<Float>,
        best_param_json -> Nullable<Text>,
        iters -> Integer,
        best_iters -> Integer,
        termination -> Text,
        error_message -> Nullable<Text>,
        finished_at -> Timestamp,
    }
}
