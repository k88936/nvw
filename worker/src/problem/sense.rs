use crate::utils;
use anyhow::Error;
use argmin::core::CostFunction;
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
            client_satellites_ephem: utils::init_client_satellites_ephem(),
            sense_radius_km: 1234.0,
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
                Ephem::from_orbit(orb, utils::gen_epochs(), Plane::EarthEquator)
            })
            .collect();

        let sense_radius_km_square = self.sense_radius_km * self.sense_radius_km;

        let coverage_score: f64 = self
            .client_satellites_ephem
            .iter()
            .flat_map(|ephem| {
                // every client
                ephem.rv(None).0.into_iter().enumerate().map(|(i, pos)| {
                    // every time point

                    let mut cost = 1024.0;

                    for dist_square in senser_ephem.iter().map(|sense_ephem| {
                        // dist collection
                        (sense_ephem.rv(None).0[i] - pos).magnitude_squared()
                    }) {
                        if dist_square < sense_radius_km_square {
                            return 0f64;
                        }
                        cost = cost + (dist_square / sense_radius_km_square).ln();
                    }
                    cost
                })
            })
            .sum();

        Ok(coverage_score)
    }
}
