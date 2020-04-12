

pub trait PrepareJob {
    fn prepare(self);
}



pub struct PrepareJobSet {
    prepare_jobs: Vec<Box<PrepareJob>>
}

impl PrepareJobSet {
    pub fn new(prepare_jobs: Vec<Box<PrepareJob>>) -> Self {
        PrepareJobSet {
            prepare_jobs
        }
    }

    pub fn prepare(&self) {

    }
}