use std::time::Duration;

#[derive(Default)]
pub struct TimeRenderResource {
    previous_update_time: Duration,
    previous_update_dt: f32,
    update_count: u64,
}

impl TimeRenderResource {
    pub fn update(
        &mut self,
        time: Duration,
    ) {
        self.previous_update_time = time;
        self.previous_update_dt = time.as_secs_f32();
        self.update_count += 1;
    }

    pub fn previous_update_time(&self) -> Duration {
        self.previous_update_time
    }

    pub fn previous_update_dt(&self) -> f32 {
        self.previous_update_dt
    }

    pub fn update_count(&self) -> u64 {
        self.update_count
    }
}
