/// Records time when created and logs amount of time passed when dropped
pub struct ScopeTimer<'a> {
    start_time: std::time::Instant,
    name: &'a str,
}

impl<'a> ScopeTimer<'a> {
    /// Records the current time. When dropped, the amount of time passed will be logged.
    #[allow(unused_must_use)]
    pub fn new(name: &'a str) -> Self {
        ScopeTimer {
            start_time: std::time::Instant::now(),
            name,
        }
    }
}

impl<'a> Drop for ScopeTimer<'a> {
    fn drop(&mut self) {
        let end_time = std::time::Instant::now();
        log::info!(
            "ScopeTimer {}: {}",
            self.name,
            (end_time - self.start_time).as_micros() as f64 / 1000.0
        )
    }
}

/// Useful for cases where you want to do something once per time interval.
#[derive(Default)]
pub struct PeriodicEvent {
    last_time_triggered: Option<std::time::Instant>,
}

impl PeriodicEvent {
    /// Call try_take_event to see if the required time has elapsed. It will return true only once
    /// enough time has passed since it last returned true.
    pub fn try_take_event(
        &mut self,
        current_time: std::time::Instant,
        wait_duration: std::time::Duration,
    ) -> bool {
        match self.last_time_triggered {
            None => {
                self.last_time_triggered = Some(current_time);
                true
            }
            Some(last_time_triggered) => {
                if current_time - last_time_triggered >= wait_duration {
                    self.last_time_triggered = Some(current_time);
                    true
                } else {
                    false
                }
            }
        }
    }
}

#[derive(Copy, Clone)]
pub enum SimulationTimePauseReason {
    Editor = 1,
    User = 2,
}

enum TimeOp {
    SetPaused(bool, SimulationTimePauseReason),
    ResetSimulationTime,
}

// For now just wrap the input helper that skulpin provides
pub struct TimeResource {
    pub time_state: TimeState,
    pub simulation_time: TimeContext,
    pub log_fps_event: PeriodicEvent,
    pub simulation_pause_flags: u8, // No flags set means simulation is not paused
    pending_time_ops: Vec<TimeOp>,
}

impl TimeResource {
    /// Create a new TimeState. Default is not allowed because the current time affects the object
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        TimeResource {
            time_state: TimeState::new(),
            simulation_time: TimeContext::new(),
            log_fps_event: Default::default(),
            simulation_pause_flags: 0,
            pending_time_ops: Default::default(),
        }
    }

    pub fn system_time(&self) -> &TimeContext {
        self.time_state.app_time_context()
    }

    pub fn game_time(&self) -> &TimeContext {
        &self.simulation_time
    }

    pub fn set_simulation_time_paused(
        &mut self,
        paused: bool,
        reason: SimulationTimePauseReason,
    ) {
        let before = self.is_simulation_paused();
        if paused {
            self.simulation_pause_flags |= (reason as u8);
        } else {
            self.simulation_pause_flags &= !(reason as u8);
        }
        let after = self.is_simulation_paused();
        if before != after {
            log::info!("Simulation pause state change {} -> {}", before, after);
        }
    }

    pub fn reset_simulation_time(&mut self) {
        self.simulation_time = TimeContext::new();
        log::info!("Simulation time reset");
    }

    pub fn is_simulation_paused(&self) -> bool {
        self.simulation_pause_flags != 0
    }

    pub fn advance_time(&mut self) {
        self.time_state.update();
        if !self.is_simulation_paused() {
            self.simulation_time
                .update(self.time_state.previous_update_time());
        }
    }

    pub fn enqueue_set_simulation_time_paused(
        &mut self,
        paused: bool,
        reason: SimulationTimePauseReason,
    ) {
        self.pending_time_ops
            .push(TimeOp::SetPaused(paused, reason));
    }

    pub fn enqueue_reset_simulation_time(&mut self) {
        self.pending_time_ops.push(TimeOp::ResetSimulationTime);
    }

    pub fn process_time_ops(&mut self) {
        let time_ops: Vec<_> = self.pending_time_ops.drain(..).collect();
        for time_op in time_ops {
            match time_op {
                TimeOp::SetPaused(paused, reason) => {
                    self.set_simulation_time_paused(paused, reason)
                }
                TimeOp::ResetSimulationTime => self.reset_simulation_time(),
            }
        }
    }
}

use std::time;

const NANOS_PER_SEC: u32 = 1_000_000_000;

/// Contains the global time information (such as time when app was started.) There is also a
/// time context that is continuously updated
#[derive(Clone)]
pub struct TimeState {
    app_start_system_time: time::SystemTime,
    app_start_instant: time::Instant,

    // Save the instant captured during previous update
    previous_update_instant: time::Instant,

    // This contains each context that we support. This will likely be removed in a future version
    // of skulpin
    app_time_context: TimeContext,
}

impl TimeState {
    /// Create a new TimeState. Default is not allowed because the current time affects the object
    #[allow(clippy::new_without_default)]
    pub fn new() -> TimeState {
        let now_instant = time::Instant::now();
        let now_system_time = time::SystemTime::now();

        TimeState {
            app_start_system_time: now_system_time,
            app_start_instant: now_instant,
            previous_update_instant: now_instant,
            app_time_context: TimeContext::new(),
        }
    }

    /// Call every frame to capture time passing and update values
    pub fn update(&mut self) {
        // Determine length of time since last tick
        let now_instant = time::Instant::now();
        let elapsed = now_instant - self.previous_update_instant;
        self.previous_update_instant = now_instant;
        self.app_time_context.update(elapsed);
    }

    /// System time that the application started
    pub fn app_start_system_time(&self) -> &time::SystemTime {
        &self.app_start_system_time
    }

    /// rust Instant object captured when the application started
    pub fn app_start_instant(&self) -> &time::Instant {
        &self.app_start_instant
    }

    /// Get the app time context.
    pub fn app_time_context(&self) -> &TimeContext {
        &self.app_time_context
    }

    /// Duration of time passed
    pub fn total_time(&self) -> time::Duration {
        self.app_time_context.total_time
    }

    /// `std::time::Instant` object captured at the start of the most recent update
    pub fn current_instant(&self) -> time::Instant {
        self.app_time_context.current_instant
    }

    /// duration of time passed during the previous update
    pub fn previous_update_time(&self) -> time::Duration {
        self.app_time_context.previous_update_time
    }

    /// previous update time in f32 seconds
    pub fn previous_update_dt(&self) -> f32 {
        self.app_time_context.previous_update_dt
    }

    /// estimate of updates per second
    pub fn updates_per_second(&self) -> f32 {
        self.app_time_context.updates_per_second
    }

    /// estimate of updates per second smoothed over time
    pub fn updates_per_second_smoothed(&self) -> f32 {
        self.app_time_context.updates_per_second_smoothed
    }

    /// Total number of updates
    pub fn update_count(&self) -> u64 {
        self.app_time_context.update_count
    }
}

/// Tracks time passing, this is separate from the "global" `TimeState` since it would be
/// possible to track a separate "context" of time, for example "unpaused" time in a game
#[derive(Copy, Clone)]
pub struct TimeContext {
    total_time: time::Duration,
    current_instant: time::Instant,
    previous_update_time: time::Duration,
    previous_update_dt: f32,
    updates_per_second: f32,
    updates_per_second_smoothed: f32,
    update_count: u64,
}

impl TimeContext {
    /// Create a new TimeState. Default is not allowed because the current time affects the object
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let now_instant = time::Instant::now();
        let zero_duration = time::Duration::from_secs(0);
        TimeContext {
            total_time: zero_duration,
            current_instant: now_instant,
            previous_update_time: zero_duration,
            previous_update_dt: 0.0,
            updates_per_second: 0.0,
            updates_per_second_smoothed: 0.0,
            update_count: 0,
        }
    }

    /// Call to capture time passing and update values
    pub fn update(
        &mut self,
        elapsed: std::time::Duration,
    ) {
        self.total_time += elapsed;
        self.current_instant += elapsed;
        self.previous_update_time = elapsed;

        // this can eventually be replaced with as_float_secs
        let dt =
            (elapsed.as_secs() as f32) + (elapsed.subsec_nanos() as f32) / (NANOS_PER_SEC as f32);

        self.previous_update_dt = dt;

        let fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };

        //TODO: Replace with a circular buffer
        const SMOOTHING_FACTOR: f32 = 0.95;
        self.updates_per_second = fps;
        self.updates_per_second_smoothed = (self.updates_per_second_smoothed * SMOOTHING_FACTOR)
            + (fps * (1.0 - SMOOTHING_FACTOR));

        self.update_count += 1;
    }

    /// Duration of time passed in this time context
    pub fn total_time(&self) -> time::Duration {
        self.total_time
    }

    /// `std::time::Instant` object captured at the start of the most recent update in this time
    /// context
    pub fn current_instant(&self) -> time::Instant {
        self.current_instant
    }

    /// duration of time passed during the previous update
    pub fn previous_update_time(&self) -> time::Duration {
        self.previous_update_time
    }

    /// previous update time in f32 seconds
    pub fn previous_update_dt(&self) -> f32 {
        self.previous_update_dt
    }

    /// estimate of updates per second
    pub fn updates_per_second(&self) -> f32 {
        self.updates_per_second
    }

    /// estimate of updates per second smoothed over time
    pub fn updates_per_second_smoothed(&self) -> f32 {
        self.updates_per_second_smoothed
    }

    /// Total number of update in this time context
    pub fn update_count(&self) -> u64 {
        self.update_count
    }
}
