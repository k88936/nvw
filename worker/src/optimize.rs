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

#[test]
fn optimize_test() {
    // 0: a_km 6300-oo 1: e 0-1 2:inc_degree 0-180 3:raan_degree 0-360 4: argp_degree 0-360 5: nu_degree 0-360
    let single_bounds_min = vec![20000f64, 0f64, 0f64, 0f64, 0f64, 0f64];
    let single_bounds_max = vec![45000f64, 0.1f64, 180f64, 360f64, 360f64, 360f64];
    let satellite_num = 16;
    let param = TaskPayload {
        swarm_scale: 128,
        param_bounds_min: iter::repeat_n(single_bounds_min, satellite_num)
            .flatten()
            .collect(),
        param_bounds_max: iter::repeat_n(single_bounds_max, satellite_num)
            .flatten()
            .collect(),
        max_iters: 64,
    };
    match run_optimization(&param) {
        SolverResult::Success(opt, metrics) => {
            println!("optimization successful: {:?} {:?}", opt, metrics);
            let mut plotter = OrbitPlotter::new();
            let orb = Orbit::from_vectors(
                EARTH,
                CLIENT_SATELLITES[0].r_km,
                CLIENT_SATELLITES[0].v_km_s,
            );
            plotter.plot(&orb, Some("Client"));
            let orbit_params = opt.best_param.clone();
            let coe = ClassicalElements {
                p_km: orbit_params[0] * (1.0 - orbit_params[1] * orbit_params[1]), // Semi-latus rectum from a and e
                ecc: orbit_params[1].clone(),
                inc_rad: orbit_params[2].to_radians(),
                raan_rad: orbit_params[3].to_radians(),
                argp_rad: orbit_params[4].to_radians(),
                nu_rad: orbit_params[5].to_radians(),
            };
            let orb = Orbit::from_classical(EARTH, coe);
            plotter.plot(&orb, Some("sensor"));
            let path = std::path::Path::new("test_plot.png");
            plotter.save_2d(&path).unwrap();
        }
        SolverResult::Failure(opt, metrics) => {
            println!("optimization failed: {:?} {:?}", opt, metrics);
        }
    }
}
