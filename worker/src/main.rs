pub mod data;
mod optimize;
pub mod problem;
pub mod solver;
pub mod utils;

use std::{env, iter, time::Duration};

use crate::data::CLIENT_SATELLITES;
use crate::problem::sense::SenseProblem;
use anyhow::Result;
use argmin::core::Executor;
use argmin::core::observers::ObserverMode;
use argmin::solver::particleswarm::ParticleSwarm;
use argmin_observer_slog::SlogLogger;
use chrono::Utc;
use nalgebra::{DVector, OMatrix};
use poliastrs::bodies::EARTH;
use poliastrs::core::elements::ClassicalElements;
use poliastrs::plotting::orbit_plotter::OrbitPlotter;
use poliastrs::twobody::orbit::Orbit;
use proto::{ClaimTaskRequest, ClaimTaskResponse, SubmitTaskResultRequest, TaskPayload, Version};
use tracing::{error, info, warn};
use uuid::Uuid;

// #[tokio::main]
// async fn main() -> Result<()> {
//     tracing_subscriber::fmt().init();
//
//     let server_url =
//         env::var("SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
//     let worker_id = env::var("WORKER_ID")
//         .ok()
//         .and_then(|v| Uuid::parse_str(&v).ok())
//         .unwrap_or_else(Uuid::new_v4);
//     let poll_interval_ms: u64 = 4096;
//     let bearer_token = env::var("BEARER_TOKEN").unwrap_or("k88936".into());
//
//     let mut headers = reqwest::header::HeaderMap::new();
//     let mut auth_value = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", bearer_token))?;
//     auth_value.set_sensitive(true);
//     headers.insert(reqwest::header::AUTHORIZATION, auth_value);
//
//     let client = reqwest::Client::builder()
//         .default_headers(headers)
//         .build()?;
//     info!(
//         worker_id = %worker_id,
//         server_url = %server_url,
//         poll_interval_ms,
//         "worker started"
//     );
//     // ensure version compatibile
//     loop {
//         let version_resp = client
//             .get(&format!("{}/api/version", server_url))
//             .send()
//             .await?;
//         if !version_resp.status().is_success() {
//             error!("version check failed: {}", version_resp.status());
//             tokio::time::sleep(Duration::from_secs(poll_interval_ms)).await;
//             continue;
//         }
//         let version_resp: Version = version_resp.json().await?;
//         let worker_version = Version::default();
//         if version_resp.major != worker_version.major || version_resp.minor != worker_version.minor {
//             panic!("worker version outdated. current: {:?} ,server: {:?}", worker_version,version_resp)
//         }
//         break;
//     }
//
//     loop {
//         info!(worker_id = %worker_id, "polling for task");
//         let claim_resp = client
//             .post(format!("{}/v1/tasks/claim", server_url))
//             .json(&ClaimTaskRequest { worker_id })
//             .send()
//             .await?;
//
//         if !claim_resp.status().is_success() {
//             error!("claim request failed: {}", claim_resp.status());
//             tokio::time::sleep(Duration::from_millis(poll_interval_ms)).await;
//             continue;
//         }
//
//         let claimed: ClaimTaskResponse = claim_resp.json().await?;
//         let Some(lease) = claimed.lease else {
//             info!(worker_id = %worker_id, "no task available");
//             tokio::time::sleep(Duration::from_millis(poll_interval_ms)).await;
//             continue;
//         };
//         info!(
//             worker_id = %worker_id,
//             task_id = %lease.task_id,
//             lease_id = %lease.lease_id,
//             attempt = lease.attempt,
//             "task claimed"
//         );
//
//         let (outcome, success, failure, metrics) = match run_optimization(&lease.payload) {
//             SolverResult::Success(ok, metrics) => (proto::ResultOutcome::Succeeded, Some(ok), None, metrics),
//             SolverResult::Failure(err, metrics) => (proto::ResultOutcome::Failed, None, Some(err), metrics),
//         };
//         match outcome {
//             proto::ResultOutcome::Succeeded => info!(
//                 worker_id = %worker_id,
//                 task_id = %lease.task_id,
//                 iters = metrics.iters,
//                 "optimization succeeded"
//             ),
//             proto::ResultOutcome::Failed => warn!(
//                 worker_id = %worker_id,
//                 task_id = %lease.task_id,
//                 iters = metrics.iters,
//                 "optimization failed"
//             ),
//         }
//
//         let submit_body = SubmitTaskResultRequest {
//             task_id: lease.task_id,
//             lease_id: lease.lease_id,
//             worker_id,
//             outcome,
//             metrics,
//             success,
//             failure,
//             finished_at: Utc::now(),
//         };
//
//         let submit_resp = client
//             .post(format!("{}/v1/tasks/result", server_url))
//             .json(&submit_body)
//             .send()
//             .await?;
//         if !submit_resp.status().is_success() {
//             error!("submit request failed: {}", submit_resp.status());
//         } else {
//             info!(
//                 worker_id = %worker_id,
//                 task_id = %lease.task_id,
//                 lease_id = %lease.lease_id,
//                 status = %submit_resp.status(),
//                 "result submitted"
//             );
//         }
//     }
// }
fn main() {
    // 0: a_km 6300-oo 1: e 0-1 2:inc_degree 0-180 3:raan_degree 0-360 4: argp_degree 0-360 5: nu_degree 0-360
    let single_bounds_min = vec![20000f64, 0f64, 0f64, 0f64, 0f64, 0f64];
    let single_bounds_max = vec![45000f64, 0.1f64, 180f64, 360f64, 360f64, 360f64];
    let satellite_num = 10;
    let swarm_scale = 256;
    let max_iters = 256;
    // let param = TaskPayload {
    //     swarm_scale: 128,
    //     param_bounds_min: single_bounds_min,
    //     param_bounds_max: single_bounds_max,
    //     max_iters: 128,
    // };

    let mut problem = SenseProblem::default();
    let mut cached_vars: DVector<f64> = DVector::from_vec(vec![]);

    for _ in 0..satellite_num {
        let lower = DVector::from_vec(single_bounds_min.clone());
        let upper = DVector::from_vec(single_bounds_max.clone());

        problem = SenseProblem::from_previous(problem.clone(), cached_vars.clone());

        let solver = ParticleSwarm::new((lower, upper), swarm_scale);
        let result = Executor::new(problem.clone(), solver)
            .configure(|state| state.max_iters(max_iters as u64))
            .add_observer(SlogLogger::term(), ObserverMode::Always)
            .run()
            .expect("optimize failed");
        let state = result.state();
        let best = state
            .best_individual
            .as_ref()
            .map(|p| p.position.clone())
            .unwrap();
        let cost = state.best_individual.as_ref().map(|p| p.cost).unwrap();
        println!("opt result: {:?} {:?}", cost, best);
        cached_vars = best;
    }
}
