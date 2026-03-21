use crate::problem::sense::SubProblem;
use argmin::core::observers::ObserverMode;
use argmin::core::{CostFunction, Executor};
use argmin::solver::particleswarm::ParticleSwarm;
use argmin_observer_slog::SlogLogger;
use nalgebra::DVector;
use std::cmp::Ordering;

#[derive(Clone, Debug)]
pub struct BeamSearchConfig {
    pub beam_width: usize,
    pub max_depth: usize,
    pub swarm_scale: usize,
    pub max_iters: u64,
    pub bounds_min: Vec<f64>,
    pub bounds_max: Vec<f64>,
    pub target_cost: f64,
}

#[derive(Clone)]
struct BeamNode<P> {
    problem: P,
    params: Vec<DVector<f64>>,
    cost: f64,
}

#[derive(Debug, Clone)]
pub struct BeamSearchResult {
    pub cost: f64,
    pub params: Vec<DVector<f64>>,
}

pub fn beam_pso<P>(initial_problem: P, config: BeamSearchConfig) -> Result<BeamSearchResult, String>
where
    P: CostFunction<Param = DVector<f64>, Output = f64>
        + SubProblem<P>
        + Clone
        + Send
        + Sync
        + 'static,
{
    let lower = DVector::from_vec(config.bounds_min.clone());
    let upper = DVector::from_vec(config.bounds_max.clone());
    let init_cost = initial_problem.get_score();

    let mut candidates = vec![BeamNode {
        problem: initial_problem,
        params: vec![],
        cost: init_cost,
    }];

    for i in 0..config.max_depth {
        let mut next_candidates = Vec::new();

        println!(
            "Iteration {}: Expanding {} candidates",
            i + 1,
            candidates.len()
        );

        for candidate in &candidates {
            // Branch factor: run PSO multiple times to find diverse next steps
            for _ in 0..config.beam_width {
                let solver = ParticleSwarm::new((lower.clone(), upper.clone()), config.swarm_scale);
                let result = Executor::new(candidate.problem.clone(), solver)
                    .configure(|state| state.max_iters(config.max_iters))
                    .add_observer(SlogLogger::term(), ObserverMode::Always)
                    .run();

                match result {
                    Ok(res) => {
                        let state = res.state();

                        if let Some(best_param) =
                            state.best_individual.as_ref().map(|p| p.position.clone())
                        {
                            let cost = state
                                .best_individual
                                .as_ref()
                                .map(|p| p.cost)
                                .unwrap_or(f64::MAX);

                            let next_problem =
                                P::from_previous(candidate.problem.clone(), best_param.clone());
                            let mut next_params = candidate.params.clone();
                            next_params.push(best_param);

                            next_candidates.push(BeamNode {
                                problem: next_problem,
                                params: next_params,
                                cost,
                            });
                        }
                    }
                    Err(e) => {
                        eprintln!("Optimization failed: {}", e);
                    }
                }
            }
        }

        if next_candidates.is_empty() {
            println!("No valid candidates found in iteration {}", i + 1);
            break;
        }

        // Sort by cost (ascending) and keep top K
        next_candidates.sort_by(|a, b| a.cost.partial_cmp(&b.cost).unwrap_or(Ordering::Equal));

        // Truncate to beam width
        candidates = next_candidates
            .into_iter()
            .take(config.beam_width)
            .collect();

        if let Some(best) = candidates.first() {
            println!("Best cost at iteration {}: {}", i + 1, best.cost);
            if best.cost < config.target_cost {
                println!("Target cost reached!");
                return Ok(BeamSearchResult {
                    cost: best.cost,
                    params: best.params.clone(),
                });
            }
        }
    }

    if let Some(best) = candidates.first() {
        Ok(BeamSearchResult {
            cost: best.cost,
            params: best.params.clone(),
        })
    } else {
        Err("No solution found".to_string())
    }
}
