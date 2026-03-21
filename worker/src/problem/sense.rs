use crate::utils;
use crate::utils::init_client_satellites_ephem;
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
    pub check_points: Vec<(usize, Vector3<f64>)>,
    pub sense_radius_km: f64,
}

impl SenseProblem {
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

impl SubProblem<SenseProblem> for SenseProblem {
    fn from_previous(problem: SenseProblem, staged_vars: DVector<f64>) -> Self {
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
    fn get_scale(&self) -> f64 {
        self.check_points.len() as f64
    }
}

pub trait SubProblem<T> {
    fn from_previous(previous: T, sub_solved: DVector<f64>) -> Self;
    fn get_scale(&self) -> f64;
}

impl Default for SenseProblem {
    fn default() -> Self {
        Self {
            check_points: init_client_satellites_ephem()
                .iter()
                .flat_map(|ephem| ephem.rv(None).0.into_iter().enumerate())
                .collect(),
            sense_radius_km: 5000.0,
        }
    }
}

impl CostFunction for SenseProblem {
    type Param = DVector<f64>;
    type Output = f64;

    fn cost(&self, param: &Self::Param) -> Result<Self::Output, Error> {
        let senser_ephem = Self::collect_senser_ephems(param);

        let sense_radius_km_square = self.sense_radius_km * self.sense_radius_km;
        let coverage_score: usize = self
            .check_points
            .iter()
            .map(|(i, pos)| {
                match senser_ephem.iter().any(|sense_ephem| {
                    (sense_ephem.rv(None).0[*i] - pos).magnitude_squared() < sense_radius_km_square
                }) {
                    true => 0,
                    false => 1,
                }
            })
            .sum();

        Ok(coverage_score as f64)
    }
}
