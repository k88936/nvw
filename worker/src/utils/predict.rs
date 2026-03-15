use poliastrs::bodies::EARTH;
use poliastrs::twobody::orbit::Orbit;
use poliastrs::core::elements::ClassicalElements;
use poliastrs::frames::Plane;
use poliastrs::ephem::Ephem;
use chrono::{TimeZone, Utc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Migration Demo: Computing relative distance between two orbits");

    // Time handling: Convert 2024-01-01 to TDB seconds from J2000
    // J2000 epoch is 2000-01-01 12:00:00 UTC
    let j2000 = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).unwrap();
    let epoch_date = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();

    // Calculate seconds from J2000
    // Note: In a rigorous application, we should account for leap seconds (TAI-UTC)
    // and TT-TDB differences, but for this example, we'll use a simple difference.
    let epoch_tdb = (epoch_date - j2000).num_milliseconds() as f64 / 1000.0;

    println!("Epoch (J2000 TDB seconds): {:.3}", epoch_tdb);

    // Define initial orbits (Example values)
    // Orbit 1: generic LEO
    let coe1 = ClassicalElements {
        p_km: 7000.0 * (1.0 - 0.001 * 0.001), // Semi-latus rectum from a=7000, e=0.001
        ecc: 0.001,
        inc_rad: 53.0_f64.to_radians(),
        raan_rad: 10.0_f64.to_radians(),
        argp_rad: 0.0,
        nu_rad: 0.0,
    };

    let orb1 = Orbit::from_classical_at(
        EARTH,
        coe1,
        epoch_tdb,
        Plane::EarthEquator,
    ).map_err(|e| format!("OrbitError: {:?}", e))?;

    // Orbit 2: slightly different LEO
    let coe2 = ClassicalElements {
        p_km: 7200.0 * (1.0 - 0.01 * 0.01),
        ecc: 0.01,
        inc_rad: 54.0_f64.to_radians(),
        raan_rad: 15.0_f64.to_radians(),
        argp_rad: 20.0_f64.to_radians(),
        nu_rad: 0.0,
    };

    let orb2 = Orbit::from_classical_at(
        EARTH,
        coe2,
        epoch_tdb,
        Plane::EarthEquator,
    ).map_err(|e| format!("OrbitError: {:?}", e))?;

    // Create shared time span: 0 to 3600 seconds, 100 steps
    let steps = 100;
    let duration = 3600.0;
    let dt = duration / (steps as f64 - 1.0);

    let epochs: Vec<f64> = (0..steps)
        .map(|i| epoch_tdb + i as f64 * dt)
        .collect();

    // Generate Ephemerides
    let ephem1 = Ephem::from_orbit(orb1, epochs.clone(), Plane::EarthEquator);
    let ephem2 = Ephem::from_orbit(orb2, epochs.clone(), Plane::EarthEquator);

    // Compute distances
    let (r1_vecs, _) = ephem1.rv(None);
    let (r2_vecs, _) = ephem2.rv(None);

    // Calculate norm of difference vector for each step
    let distances: Vec<f64> = r1_vecs.iter().zip(r2_vecs.iter())
        .map(|(r1, r2)| (r1 - r2).norm())
        .collect();

    // Print first few results
    println!("\nCalculated {} distances:", distances.len());
    for (i, dist) in distances.iter().take(5).enumerate() {
        println!("T+{:.0}s: {:.3} km", i as f64 * dt, dist);
    }
    println!("...");
    for (i, dist) in distances.iter().skip(steps - 5).enumerate() {
        println!("T+{:.0}s: {:.3} km", (steps - 5 + i) as f64 * dt, dist);
    }

    // Stats
    let min_dist = distances.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_dist = distances.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let mean_dist: f64 = distances.iter().sum::<f64>() / distances.len() as f64;

    println!("\nStatistics:");
    println!("Min distance: {:.3} km", min_dist);
    println!("Max distance: {:.3} km", max_dist);
    println!("Mean distance: {:.3} km", mean_dist);

    Ok(())
}
