use crate::data::CLIENT_SATELLITES;
use crate::problem::sense::SenseProblem;
use argmin::core::Executor;
use argmin::core::observers::ObserverMode;
use argmin::solver::particleswarm::ParticleSwarm;
use argmin_observer_slog::SlogLogger;
use nalgebra::DVector;
use poliastrs::bodies::EARTH;
use poliastrs::core::elements::ClassicalElements;
use poliastrs::plotting::orbit_plotter::OrbitPlotter;
use poliastrs::twobody::orbit::Orbit;
use proto::{FailedOptimization, SuccessfulOptimization, TaskPayload, TaskRunMetrics};
use std::iter;
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
                        best_cost: cost,
                        best_param: best.iter().map(|v| v.clone()).collect(),
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

