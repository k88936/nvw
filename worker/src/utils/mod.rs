use poliastrs::ephem::Ephem;
use poliastrs::twobody::orbit::Orbit;
use poliastrs::bodies::EARTH;
use poliastrs::frames::Plane;
use chrono::{TimeZone, Utc};
use crate::data::CLIENT_SATELLITES;

pub static TIME_STEP: usize = 128;

pub fn init_client_satellites_ephem() -> Vec<Ephem> {
    CLIENT_SATELLITES
        .iter()
        .map(|satellite| {
            let orb = Orbit::from_vectors(EARTH, satellite.r_km, satellite.v_km_s);
            Ephem::from_orbit(orb, gen_epochs(), Plane::EarthEquator)
        })
        .collect()
}

pub fn gen_epochs() -> Vec<f64> {
    // Time handling: Convert 2026-01-01 to TDB seconds from J2000
    // J2000 epoch is 2000-01-01 12:00:00 UTC
    let j2000 = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).unwrap();
    let epoch_date = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let epoch_tdb = (epoch_date - j2000).num_milliseconds() as f64 / 1000.0;

    let duration = (60 * 60 * TIME_STEP) as f64;
    let dt = duration / (TIME_STEP as f64 - 1.0);

    let epochs: Vec<f64> = (0..TIME_STEP).map(|i| epoch_tdb + i as f64 * dt).collect();
    epochs
}