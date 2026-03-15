use argmin::core::{CostFunction, Error, Executor};
use argmin::solver::particleswarm::ParticleSwarm;
use nalgebra::DVector;
use proto::{FailedOptimization, SuccessfulOptimization, TaskPayload, TaskRunMetrics};
use tracing::{info, warn};

struct Sphere;

impl CostFunction for Sphere {
    type Param = DVector<f64>;
    type Output = f64;

    fn cost(&self, param: &Self::Param) -> Result<Self::Output, Error> {
        Ok(param.iter().map(|v| v * v).sum())
    }
}

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
    if payload.param_count == 0 || payload.param_bounds.len() != payload.param_count {
        warn!(
            param_count = payload.param_count,
            bounds_len = payload.param_bounds.len(),
            "payload validation failed"
        );
        return SolverResult::Failure(
            FailedOptimization {
                error_message: "invalid payload bounds".to_string(),
            },
            metrics_on_fail,
        );
    }

    let lower = DVector::from_vec(payload.param_bounds.iter().map(|b| b.min as f64).collect());
    let upper = DVector::from_vec(payload.param_bounds.iter().map(|b| b.max as f64).collect());
    info!(
        dims = payload.param_count,
        max_iters = payload.max_iters,
        "starting optimization"
    );
    let solver = ParticleSwarm::new((lower, upper), 64);

    match Executor::new(Sphere, solver)
        .configure(|state| state.max_iters(payload.max_iters as u64))
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
