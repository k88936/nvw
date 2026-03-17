use crate::utils;
use crate::utils::{TIME_STEP, init_client_satellites_ephem};
use anyhow::Error;
use argmin::core::CostFunction;
use itertools::Itertools;
use nalgebra::{DVector, Vector3};
use poliastrs::bodies::EARTH;
use poliastrs::core::elements::ClassicalElements;
use poliastrs::ephem::Ephem;
use poliastrs::frames::Plane;
use poliastrs::twobody::orbit::Orbit;

#[derive(Clone)]
pub struct SenseProblem {
    check_points: Vec<(usize, Vector3<f64>)>,
    sense_radius_km: f64,
}
impl SenseProblem {
    pub fn from_previous(problem: SenseProblem, staged_vars: DVector<f64>) -> Self {
        let senser_ephem = Self::collect_senser_ephems(&staged_vars);
        let sense_radius_km_square = problem.sense_radius_km * problem.sense_radius_km;
        Self {
            check_points: problem
                .check_points
                .into_iter()
                .filter(|(i, pos)| {
                    !senser_ephem.iter().any(|sense_ephem| {
                        (sense_ephem.rv(None).0[*i] - pos).magnitude_squared()
                            < sense_radius_km_square
                    })
                })
                .collect(),
            sense_radius_km: problem.sense_radius_km,
        }
    }

    fn collect_senser_ephems(param: &DVector<f64>) -> Vec<Ephem> {
        let sensor_params = param.iter().chunks(6);
        let senser_ephem: Vec<_> = sensor_params
            .into_iter()
            .map(|x| {
                let orbit_params: Vec<_> = x.collect();
                let coe = ClassicalElements {
                    p_km: orbit_params[0] * (1.0 - orbit_params[1] * orbit_params[1]), // Semi-latus rectum from a and e
                    ecc: orbit_params[1].clone(),
                    inc_rad: orbit_params[2].to_radians(),
                    raan_rad: orbit_params[3].to_radians(),
                    argp_rad: orbit_params[4].to_radians(),
                    nu_rad: orbit_params[5].to_radians(),
                };
                let orb = Orbit::from_classical(EARTH, coe);
                Ephem::from_orbit(orb, utils::gen_epochs(), Plane::EarthEquator)
            })
            .collect();
        senser_ephem
    }
}
impl Default for SenseProblem {
    fn default() -> Self {
        Self {
            check_points: init_client_satellites_ephem()
                .iter()
                .flat_map(|ephem| ephem.rv(None).0.into_iter().enumerate())
                .collect(),
            sense_radius_km: 1234.0,
        }
    }
}

impl CostFunction for SenseProblem {
    type Param = DVector<f64>;
    type Output = f64;

    fn cost(&self, param: &Self::Param) -> Result<Self::Output, Error> {
        let senser_ephem = Self::collect_senser_ephems(param);

        let sense_radius_km_square = self.sense_radius_km * self.sense_radius_km;
        let default_lost_cost = 1024f64;

        let coverage_score: f64 = self
            .check_points
            .iter()
            // every client
            .map(|(i, pos)| {
                // every time point
                let mut cost = default_lost_cost;

                // dist collection
                for dist_square in senser_ephem
                    .iter()
                    .map(|sense_ephem| (sense_ephem.rv(None).0[*i] - pos).magnitude_squared())
                {
                    if dist_square < sense_radius_km_square {
                        return 0f64;
                    }
                    cost = cost + (dist_square / sense_radius_km_square).ln();
                }
                cost
            })
            .sum();

        Ok(
            coverage_score
                / (self.check_points.len() as f64 * default_lost_cost),
        )
    }
}
