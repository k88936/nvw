use crate::data::CLIENT_SATELLITES;
use anyhow::Error;
use argmin::core::CostFunction;
use chrono::{TimeZone, Utc};
use itertools::Itertools;
use nalgebra::DVector;
use poliastrs::bodies::EARTH;
use poliastrs::core::elements::ClassicalElements;
use poliastrs::ephem::Ephem;
use poliastrs::frames::Plane;
use poliastrs::twobody::orbit::Orbit;

pub struct SenseProblem {
    client_satellites_ephem: Vec<Ephem>,
    sense_radius_km: f64,
}
impl Default for SenseProblem {
    fn default() -> Self {
        Self {
            client_satellites_ephem: init_client_satellites_ephem(),
            sense_radius_km: 11234.0,
        }
    }
}
impl CostFunction for SenseProblem {
    type Param = DVector<f64>;
    type Output = f64;

    fn cost(&self, param: &Self::Param) -> Result<Self::Output, Error> {
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
                Ephem::from_orbit(orb, gen_epochs(), Plane::EarthEquator)
            })
            .collect();

        let sense_radius_km_square = self.sense_radius_km * self.sense_radius_km;

        let coverage_score: i32 = self
            .client_satellites_ephem
            .iter()
            .map(|ephem| {
                let one_client_score: i32 = ephem
                    .rv(None)
                    .0
                    .iter()
                    .enumerate()
                    .map(|(i, pos)| {
                        match senser_ephem.iter().any(|sense_ephem| {
                            let d1 = sense_ephem.rv(None).0[i];
                            let d2 = d1 - pos;
                            let d3 = d2.magnitude_squared();
                            d3 < sense_radius_km_square
                        }) {
                            true => 0,
                            false => 1,
                        }
                    })
                    .sum();
                one_client_score
            })
            .sum();

        Ok(coverage_score as f64 / (TIME_STEP * self.client_satellites_ephem.len()) as f64)
    }
}

pub static TIME_STEP: usize = 84;
fn init_client_satellites_ephem() -> Vec<Ephem> {
    CLIENT_SATELLITES
        .iter()
        .map(|satellite| {
            let orb = Orbit::from_vectors(EARTH, satellite.r_km, satellite.v_km_s);
            Ephem::from_orbit(orb, gen_epochs(), Plane::EarthEquator)
        })
        .collect()
}
fn gen_epochs() -> Vec<f64> {
    // Time handling: Convert 2026-01-01 to TDB seconds from J2000
    // J2000 epoch is 2000-01-01 12:00:00 UTC
    let j2000 = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).unwrap();
    let epoch_date = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let epoch_tdb = (epoch_date - j2000).num_milliseconds() as f64 / 1000.0;

    let duration = (60 * 60 * 2 * TIME_STEP) as f64;
    let dt = duration / (TIME_STEP as f64 - 1.0);

    let epochs: Vec<f64> = (0..TIME_STEP).map(|i| epoch_tdb + i as f64 * dt).collect();
    epochs
}
