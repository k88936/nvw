// Copyright 2018-2024 argmin developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! # Particle Swarm Optimization (PSO)
//!
//! Canonical implementation of the particle swarm optimization method as outlined in \[0\] in
//! chapter II, section A.
//!
//! For details see [`SteppedParticleSwarm`].
//!
//! ## References
//!
//! \[0\] Zambrano-Bigiarini, M. et.al. (2013): Standard Particle Swarm Optimisation 2011 at
//! CEC-2013: A baseline for future PSO improvements. 2013 IEEE Congress on Evolutionary
//! Computation. <https://doi.org/10.1109/CEC.2013.6557848>
//!
//! \[1\] <https://en.wikipedia.org/wiki/Particle_swarm_optimization>

use argmin::core::{
    ArgminFloat, CostFunction, Error, KV, PopulationState, Problem, Solver, SyncAlias,
};
use argmin::{argmin_error, argmin_error_closure, float};
use argmin_math::{ArgminAdd, ArgminMinMax, ArgminMul, ArgminRandom, ArgminSub, ArgminZeroLike};
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use crate::solver::SteppedProblem;

/// # Particle Swarm Optimization (PSO)
///
/// Canonical implementation of the particle swarm optimization method as outlined in \[0\] in
/// chapter II, section A.
///
/// The `rayon` feature enables parallel computation of the cost function. This can be beneficial
/// for expensive cost functions, but may cause a drop in performance for cheap cost functions. Be
/// sure to benchmark both parallel and sequential computation.
///
/// ## Requirements on the optimization problem
///
/// The optimization problem is required to implement [`CostFunction`].
///
/// ## References
///
/// \[0\] Zambrano-Bigiarini, M. et.al. (2013): Standard Particle Swarm Optimisation 2011 at
/// CEC-2013: A baseline for future PSO improvements. 2013 IEEE Congress on Evolutionary
/// Computation. <https://doi.org/10.1109/CEC.2013.6557848>
///
/// \[1\] <https://en.wikipedia.org/wiki/Particle_swarm_optimization>
#[derive(Clone, Serialize, Deserialize)]
pub struct SteppedParticleSwarm<P, F, R> {
    /// Inertia weight
    weight_inertia: F,
    /// Cognitive acceleration coefficient
    weight_cognitive: F,
    /// Social acceleration coefficient
    weight_social: F,
    /// Bounds on parameter space
    bounds: (P, P),
    /// Number of particles
    num_particles: usize,
    /// Random number generator
    rng_generator: R,
}

impl<P, F> SteppedParticleSwarm<P, F, rand::rngs::StdRng>
where
    P: Clone + SyncAlias + ArgminSub<P, P> + ArgminMul<F, P> + ArgminRandom + ArgminZeroLike,
    F: ArgminFloat,
{
    /// Construct a new instance of `ParticleSwarm`
    ///
    /// Takes the number of particles and bounds on the search space as inputs. `bounds` is a tuple
    /// `(lower_bound, upper_bound)`, where `lower_bound` and `upper_bound` are of the same type as
    /// the position of a particle (`P`) and of the same length as the problem as dimensions.
    ///
    /// The inertia weight on velocity and the social and cognitive acceleration factors can be
    /// adapted with [`with_inertia_factor`](`ParticleSwarm::with_inertia_factor`),
    /// [`with_cognitive_factor`](`ParticleSwarm::with_cognitive_factor`) and
    /// [`with_social_factor`](`ParticleSwarm::with_social_factor`), respectively.
    ///
    /// The weights and acceleration factors default to:
    ///
    /// * inertia: `1/(2 * ln(2))`
    /// * cognitive: `0.5 + ln(2)`
    /// * social: `0.5 + ln(2)`
    ///
    /// # Example
    ///
    /// ```
    /// # use argmin::solver::particleswarm::ParticleSwarm;
    /// # let lower_bound: Vec<f64> = vec![-1.0, -1.0];
    /// # let upper_bound: Vec<f64> = vec![1.0, 1.0];
    /// let pso: ParticleSwarm<_, f64, _> = ParticleSwarm::new((lower_bound, upper_bound), 40);
    /// ```
    pub fn new(bounds: (P, P), num_particles: usize) -> Self {
        SteppedParticleSwarm {
            weight_inertia: float!(1.0f64 / (2.0 * 2.0f64.ln())),
            weight_cognitive: float!(0.5 + 2.0f64.ln()),
            weight_social: float!(0.5 + 2.0f64.ln()),
            bounds,
            num_particles,
            rng_generator: rand::rngs::StdRng::from_os_rng(),
        }
    }
}
impl<P, F, R0> SteppedParticleSwarm<P, F, R0>
where
    P: Clone + SyncAlias + ArgminSub<P, P> + ArgminMul<F, P> + ArgminRandom + ArgminZeroLike,
    F: ArgminFloat,
    R0: Rng,
{
    /// Set the random number generator
    ///
    /// Defaults to `rand::rngs::StdRng::from_os_rng()`
    ///
    /// # Example
    /// ```
    /// # use argmin::solver::particleswarm::ParticleSwarm;
    /// # use argmin::core::Error;
    /// # use rand::SeedableRng;
    /// # fn main() -> Result<(), Error> {
    /// # let lower_bound: Vec<f64> = vec![-1.0, -1.0];
    /// # let upper_bound: Vec<f64> = vec![1.0, 1.0];
    /// let pso: ParticleSwarm<_, f64, _> =
    ///     ParticleSwarm::new((lower_bound, upper_bound), 40)
    ///     .with_rng_generator(rand_xoshiro::Xoroshiro128Plus::seed_from_u64(1729));
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_rng_generator<R1: Rng>(self, generator: R1) -> SteppedParticleSwarm<P, F, R1> {
        SteppedParticleSwarm {
            weight_inertia: self.weight_inertia,
            weight_cognitive: self.weight_cognitive,
            weight_social: self.weight_social,
            bounds: self.bounds,
            num_particles: self.num_particles,
            rng_generator: generator,
        }
    }
}

impl<P, F, R> SteppedParticleSwarm<P, F, R>
where
    P: Clone + SyncAlias + ArgminSub<P, P> + ArgminMul<F, P> + ArgminRandom + ArgminZeroLike,
    F: ArgminFloat,
    R: Rng,
{
    /// Set inertia factor on particle velocity
    ///
    /// Defaults to `1/(2 * ln(2))`.
    ///
    /// # Example
    ///
    /// ```
    /// # use argmin::solver::particleswarm::ParticleSwarm;
    /// # use argmin::core::Error;
    /// # fn main() -> Result<(), Error> {
    /// # let lower_bound: Vec<f64> = vec![-1.0, -1.0];
    /// # let upper_bound: Vec<f64> = vec![1.0, 1.0];
    /// let pso: ParticleSwarm<_, f64, _> =
    ///     ParticleSwarm::new((lower_bound, upper_bound), 40).with_inertia_factor(0.5)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_inertia_factor(mut self, factor: F) -> Result<Self, Error> {
        if factor < float!(0.0) {
            return Err(argmin_error!(
                InvalidParameter,
                "`ParticleSwarm`: inertia factor must be >=0."
            ));
        }
        self.weight_inertia = factor;
        Ok(self)
    }

    /// Set cognitive acceleration factor
    ///
    /// Defaults to `0.5 + ln(2)`.
    ///
    /// # Example
    ///
    /// ```
    /// # use argmin::solver::particleswarm::ParticleSwarm;
    /// # use argmin::core::Error;
    /// # fn main() -> Result<(), Error> {
    /// # let lower_bound: Vec<f64> = vec![-1.0, -1.0];
    /// # let upper_bound: Vec<f64> = vec![1.0, 1.0];
    /// let pso: ParticleSwarm<_, f64, _> =
    ///     ParticleSwarm::new((lower_bound, upper_bound), 40).with_cognitive_factor(1.1)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_cognitive_factor(mut self, factor: F) -> Result<Self, Error> {
        if factor < float!(0.0) {
            return Err(argmin_error!(
                InvalidParameter,
                "`ParticleSwarm`: cognitive factor must be >=0."
            ));
        }
        self.weight_cognitive = factor;
        Ok(self)
    }

    /// Set social acceleration factor
    ///
    /// Defaults to `0.5 + ln(2)`.
    ///
    /// # Example
    ///
    /// ```
    /// # use argmin::solver::particleswarm::ParticleSwarm;
    /// # use argmin::core::Error;
    /// # fn main() -> Result<(), Error> {
    /// # let lower_bound: Vec<f64> = vec![-1.0, -1.0];
    /// # let upper_bound: Vec<f64> = vec![1.0, 1.0];
    /// let pso: ParticleSwarm<_, f64, _> =
    ///     ParticleSwarm::new((lower_bound, upper_bound), 40).with_social_factor(1.1)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_social_factor(mut self, factor: F) -> Result<Self, Error> {
        if factor < float!(0.0) {
            return Err(argmin_error!(
                InvalidParameter,
                "`ParticleSwarm`: social factor must be >=0."
            ));
        }
        self.weight_social = factor;
        Ok(self)
    }

    /// Initializes all particles randomly and sorts them by their cost function values
    fn initialize_particles<O: CostFunction<Param = P, Output = F> + SyncAlias>(
        &mut self,
        problem: &mut Problem<O>,
    ) -> Result<Vec<Particle<P, F>>, Error> {
        let (positions, velocities) = self.initialize_positions_and_velocities();

        let costs = problem.bulk_cost(&positions)?;

        let mut particles = positions
            .into_iter()
            .zip(velocities)
            .zip(costs)
            .map(|((p, v), c)| Particle::new(p, c, v))
            .collect::<Vec<_>>();

        // sort them, such that the first one is the best one
        particles.sort_by(|a, b| {
            a.cost
                .partial_cmp(&b.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(particles)
    }

    /// Initializes positions and velocities for all particles
    fn initialize_positions_and_velocities(&mut self) -> (Vec<P>, Vec<P>) {
        let (min, max) = &self.bounds;
        let delta = max.sub(min);
        let delta_neg = delta.mul(&float!(-1.0));

        (
            (0..self.num_particles)
                .map(|_| P::rand_from_range(min, max, &mut self.rng_generator))
                .collect(),
            (0..self.num_particles)
                .map(|_| P::rand_from_range(&delta_neg, &delta, &mut self.rng_generator))
                .collect(),
        )
    }
}

impl<O, P, F, R> Solver<O, PopulationState<Particle<P, F>, F>> for SteppedParticleSwarm<P, F, R>
where
    O: CostFunction<Param = P, Output = F> + SyncAlias,
    P: Clone
        + SyncAlias
        + ArgminAdd<P, P>
        + ArgminSub<P, P>
        + ArgminMul<F, P>
        + ArgminZeroLike
        + ArgminRandom
        + ArgminMinMax,
    F: ArgminFloat,
    R: Rng,
{
    fn name(&self) -> &str {
        "Particle Swarm Optimization"
    }

    fn init(
        &mut self,
        problem: &mut Problem<O>,
        mut state: PopulationState<Particle<P, F>, F>,
    ) -> Result<(PopulationState<Particle<P, F>, F>, Option<KV>), Error> {
        // Users can provide a population or it will be randomly created.
        let particles = match state.take_population() {
            Some(mut particles) if particles.len() == self.num_particles => {
                // sort them first
                particles.sort_by(|a, b| {
                    a.cost
                        .partial_cmp(&b.cost)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                particles
            }
            Some(particles) => {
                return Err(argmin_error!(
                    InvalidParameter,
                    format!(
                        "`ParticleSwarm`: Provided list of particles is of length {}, expected {}",
                        particles.len(),
                        self.num_particles
                    )
                ));
            }
            None => self.initialize_particles(problem)?,
        };

        Ok((
            state
                .individual(particles[0].clone())
                .cost(particles[0].cost)
                .population(particles),
            None,
        ))
    }

    /// Perform one iteration of algorithm
    fn next_iter(
        &mut self,
        problem: &mut Problem<O>,
        mut state: PopulationState<Particle<P, F>, F>,
    ) -> Result<(PopulationState<Particle<P, F>, F>, Option<KV>), Error> {
        let mut best_particle = state.take_individual().ok_or_else(argmin_error_closure!(
            PotentialBug,
            "`ParticleSwarm`: No current best individual in state."
        ))?;
        let mut best_cost = state.get_cost();
        let mut particles = state.take_population().ok_or_else(argmin_error_closure!(
            PotentialBug,
            "`ParticleSwarm`: No population in state."
        ))?;

        let zero = P::zero_like(&best_particle.position);

        let positions: Vec<_> = particles
            .iter_mut()
            .map(|p| {
                // New velocity is composed of
                // 1) previous velocity (momentum),
                // 2) motion toward particle optimum and
                // 3) motion toward global optimum.

                // ad 1)
                let momentum = p.velocity.mul(&self.weight_inertia);

                // ad 2)
                let to_optimum = p.best_position.sub(&p.position);
                let pull_to_optimum =
                    P::rand_from_range(&zero, &to_optimum, &mut self.rng_generator);
                let pull_to_optimum = pull_to_optimum.mul(&self.weight_cognitive);

                // ad 3)
                let to_global_optimum = best_particle.position.sub(&p.position);
                let pull_to_global_optimum =
                    P::rand_from_range(&zero, &to_global_optimum, &mut self.rng_generator)
                        .mul(&self.weight_social);

                p.velocity = momentum.add(&pull_to_optimum).add(&pull_to_global_optimum);
                let new_position = p.position.add(&p.velocity);

                // Limit to search window
                p.position = P::min(&P::max(&new_position, &self.bounds.0), &self.bounds.1);
                &p.position
            })
            .collect();

        let costs = problem.bulk_cost(&positions)?;

        for (p, c) in particles.iter_mut().zip(costs.into_iter()) {
            p.cost = c;

            if p.cost < p.best_cost {
                p.best_position = p.position.clone();
                p.best_cost = p.cost;

                if p.cost < best_cost {
                    best_particle.position = p.position.clone();
                    best_particle.best_position = p.position.clone();
                    best_particle.cost = p.cost;
                    best_particle.best_cost = p.cost;
                    best_cost = p.cost;
                }
            }
        }

        Ok((
            state
                .individual(best_particle)
                .cost(best_cost)
                .population(particles),
            None,
        ))
    }
}

/// A single particle
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde1", derive(Serialize, Deserialize))]
pub struct Particle<T, F> {
    /// Position of particle
    pub position: T,
    /// Velocity of particle
    velocity: T,
    /// Cost of particle
    pub cost: F,
    /// Best position of particle so far
    best_position: T,
    /// Best cost of particle so far
    best_cost: F,
}

impl<T, F> Particle<T, F>
where
    T: Clone,
    F: ArgminFloat,
{
    /// Create a new particle with a given position, cost and velocity.
    ///
    /// # Example
    ///
    /// ```
    /// # use argmin::solver::particleswarm::Particle;
    /// let particle: Particle<Vec<f64>, f64> = Particle::new(vec![0.0, 1.4], 12.0, vec![0.1, 0.5]);
    /// ```
    pub fn new(position: T, cost: F, velocity: T) -> Particle<T, F> {
        Particle {
            position: position.clone(),
            velocity,
            cost,
            best_position: position,
            best_cost: cost,
        }
    }
}