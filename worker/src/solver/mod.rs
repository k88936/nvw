pub mod stepped_particleswarm;
pub trait SteppedProblem {
    fn get_step(&self) -> usize;
    fn set_step(&mut self, step: usize);
}
