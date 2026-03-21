use crate::problem::sense::SubProblem;
use crate::utils;
use crate::utils::init_client_satellites_ephem;
use anyhow::Error;
use argmin::core::CostFunction;
use itertools::Itertools;
use nalgebra::DVector;
use poliastrs::bodies::EARTH;
use poliastrs::core::elements::ClassicalElements;
use poliastrs::ephem::Ephem;
use poliastrs::frames::Plane;
use poliastrs::twobody::orbit::Orbit;
use std::cmp::min;

//TODO
fn alpha_d_m(_d: f64) -> f64 {
    0.5
}
//TODO
fn mu_d_m(_d: f64) -> f64 {
    0.5
}
//TODO
fn sigma_d_m(_d: f64) -> f64 {
    0.5
}

//TODO
fn danger_d_m(_d: f64) -> f64 {
    1.0
}

#[derive(Clone)]
pub struct DebrisCleanProblem {
    pub debris_field: DVector<f64>,
    pub eval_bucket: DVector<f64>,
    // pub debris_d_grid: Vec<f64>,
    // pub debris_h_grid: Vec<f64>,
}
impl DebrisCleanProblem {
    fn collect_laser_ephems(param: &DVector<f64>) -> Vec<Ephem> {
        let laser_params = param.iter().chunks(6);
        let laser_ephem: Vec<_> = laser_params
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
        laser_ephem
    }
}
fn dihitoi(di: usize, hi: usize) -> usize {
    hi * D_VALS_LEN + di
}
fn rhtohi(h: f64) -> usize {
    min(
        (h - H_VALS_INIT).rem_euclid(H_VALS_STEP) as usize,
        H_VALS_LEN,
    )
}
static D_VALS_INIT: f64 = 1f64;
static D_VALS_STEP: f64 = 10f64;
static D_VALS_LEN: usize = 10;
static H_VALS_INIT: f64 = 25000f64;
static H_VALS_STEP: f64 = 500f64;
static H_VALS_LEN: usize = 10;

//TODO
static CLIENT_SIZE: f64 = 1f64;
fn orbital_velocity(altitude_km: f64) -> f64 {
    const MU_EARTH: f64 = 398_600.4418; // km³/s²
    const R_EARTH: f64 = 6_371.0; // km
    (MU_EARTH / (R_EARTH + altitude_km)).sqrt()
}
impl Default for DebrisCleanProblem {
    fn default() -> Self {
        let d_vals: Vec<_> = (0..10)
            .map(|i| i as f64 * D_VALS_STEP + D_VALS_INIT)
            .collect();
        let h_vals: Vec<_> = (0..10)
            .map(|i| i as f64 * H_VALS_STEP + H_VALS_INIT)
            .collect();

        let debris_d_size = d_vals.len();
        let debris_h_size = h_vals.len();

        let v_vals: Vec<_> = h_vals.iter().map(|h| orbital_velocity(*h)).collect();

        let mut d_field = Vec::new();
        for h in h_vals {
            for d in d_vals.clone() {
                let val =
                    alpha_d_m(d) * (-(h - mu_d_m(d)).powi(2) / (2.0 * sigma_d_m(d).powi(2))).exp();
                d_field.push(val);
            }
        }
        let d_field = DVector::from_vec(d_field);

        let mut eval_bucket = Vec::with_capacity(debris_h_size * debris_d_size);
        eval_bucket.resize(debris_d_size * debris_h_size, 0.0f64);

        let client_ephem = init_client_satellites_ephem();
        client_ephem.iter().for_each(|client| {
            for i in 0..client.epochs.len() {
                let hi = rhtohi(client.coordinates[i].magnitude());
                let v = client.velocities.as_ref().unwrap()[i];
                let v = v.magnitude() - v_vals[hi];
                let factor = v * CLIENT_SIZE;
                for (di, d) in d_vals.iter().enumerate() {
                    let i = dihitoi(di, hi);
                    eval_bucket[i] = eval_bucket[i] + factor * danger_d_m(*d);
                }
            }
        });
        let eval_bucket = DVector::from_vec(eval_bucket);

        Self {
            debris_field: d_field,
            eval_bucket,
        }
    }
}

impl CostFunction for DebrisCleanProblem {
    type Param = DVector<f64>;
    type Output = f64;
    fn cost(&self, param: &Self::Param) -> Result<Self::Output, Error> {
        let laser_ephem = Self::collect_laser_ephems(param);
        let mut d_field_reduced = self.debris_field.clone();

        Ok(d_field_reduced.dot(&self.eval_bucket))
    }
}

impl SubProblem<Self> for DebrisCleanProblem {
    fn from_previous(previous: Self, sub_solved: DVector<f64>) -> Self {}

    fn get_score(&self) -> f64 {
        self.debris_field.dot(&self.eval_bucket)
    }
}
