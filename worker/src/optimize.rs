use std::iter;
use crate::problem::sense::SenseProblem;
use argmin_observer_slog::SlogLogger;
use argmin::core::Executor;
use argmin::core::observers::ObserverMode;
use argmin::solver::particleswarm::ParticleSwarm;
use nalgebra::DVector;
use proto::{FailedOptimization, SuccessfulOptimization, TaskPayload, TaskRunMetrics};
use tracing::{info, warn};

pub enum SolverResult {
    Success(SuccessfulOptimization, TaskRunMetrics),
    Failure(FailedOptimization, TaskRunMetrics),
}

pub fn run_optimization(payload: &TaskPayload) -> SolverResult {
    let metrics_on_fail = TaskRunMetrics {
        iters: 0,
        best_iters: 0,
        termination: "solver_failed".to_string(),
    };
    if payload.swarm_scale == 0 || payload.param_bounds_min.len() != payload.param_bounds_max.len()
    {
        warn!(
            swarm_scale = payload.swarm_scale,
            min_bounds_len = payload.param_bounds_min.len(),
            max_bounds_len = payload.param_bounds_max.len(),
            "payload validation failed"
        );
        return SolverResult::Failure(
            FailedOptimization {
                error_message: "invalid payload bounds".to_string(),
            },
            metrics_on_fail,
        );
    }

    let lower = DVector::from_vec(payload.param_bounds_min.clone());
    let upper = DVector::from_vec(payload.param_bounds_max.clone());
    info!(max_iters = payload.max_iters, "starting optimization");
    let solver = ParticleSwarm::new((lower, upper), payload.swarm_scale);

    match Executor::new(SenseProblem::default(), solver)
        .configure(|state| state.max_iters(payload.max_iters as u64))
        .add_observer(SlogLogger::term(), ObserverMode::Always)
        .run()
    {
        Ok(res) => {
            let state = res.state();
            let maybe_best = state.best_individual.as_ref().map(|p| p.position.clone());
            let maybe_cost = state.best_individual.as_ref().map(|p| p.cost);
            match (maybe_best, maybe_cost) {
                (Some(best), Some(cost)) => SolverResult::Success(
                    SuccessfulOptimization {
                        best_cost: cost as f32,
                        best_param: best.iter().map(|v| *v as f32).collect(),
                    },
                    TaskRunMetrics {
                        iters: state.iter as usize,
                        best_iters: state.last_best_iter as usize,
                        termination: format!("{:?}", state.termination_status),
                    },
                ),
                _ => SolverResult::Failure(
                    FailedOptimization {
                        error_message: "solver produced no best result".to_string(),
                    },
                    TaskRunMetrics {
                        iters: state.iter as usize,
                        best_iters: state.last_best_iter as usize,
                        termination: format!("{:?}", state.termination_status),
                    },
                ),
            }
        }
        Err(e) => SolverResult::Failure(
            FailedOptimization {
                error_message: e.to_string(),
            },
            metrics_on_fail,
        ),
    }
}

#[test]
fn optimize_test() {

    // 0: a_km 6300-oo 1: e 0-1 2:inc_degree 0-180 3:raan_degree 0-360 4: argp_degree 0-360 5: nu_degree 0-360
    let single_bounds_min= vec![20000f64, 0f64, 0f64, 0f64, 0f64, 0f64];
    let single_bounds_max = vec![40000f64, 0.1f64, 180f64, 360f64, 360f64, 360f64];
    let satellite_num=16;
    let param = TaskPayload {
        swarm_scale: 300,
        param_bounds_min: iter::repeat_n(single_bounds_min,satellite_num).flatten().collect(),
        param_bounds_max: iter::repeat_n(single_bounds_max,satellite_num).flatten().collect(),
        max_iters: 300,
    };
    match run_optimization(&param) {
        SolverResult::Success(opt, metrics) => {
            println!("optimization successful: {:?} {:?}", opt, metrics);
        }
        SolverResult::Failure(opt, metrics) => {
            println!("optimization failed: {:?} {:?}", opt, metrics);
        }
    }
}
