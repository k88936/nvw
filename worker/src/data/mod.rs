#[derive(Debug, Clone)]
pub struct OrbitParam {
    pub orbit_type: &'static str,
    pub params: &'static str,
    pub a: f64,
    pub ecc: f64,
    pub inc: f64,
    pub raan: f64,
    pub argp: f64,
    pub nu: f64,
    pub m: f64,
}

#[derive(Debug, Clone)]
pub struct Satellite {
    pub name: &'static str,
    pub r_km: [f64; 3],
    pub v_km_s: [f64; 3],
    pub by_fly_orbits: &'static [OrbitParam],
}
include!("../../../res/beidou.rs");
