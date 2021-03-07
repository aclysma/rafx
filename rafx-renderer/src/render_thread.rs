use super::render_frame_job::RenderFrameJobResult;
use super::RenderFrameJob;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use rafx_api::RafxPresentableFrame;
use rafx_framework::RenderResources;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

enum RenderThreadMessage {
    Render(RenderFrameJob, RafxPresentableFrame),
    Finish,
}

pub struct RenderThread {
    join_handle: Option<JoinHandle<()>>,
    job_tx: Sender<RenderThreadMessage>,

    result_rx: Receiver<RenderFrameJobResult>,
    expecting_result: bool,

    render_resources: Arc<Mutex<RenderResources>>,
}

impl RenderThread {
    pub fn render_resources(&self) -> &Arc<Mutex<RenderResources>> {
        &self.render_resources
    }

    pub fn start(render_resources: RenderResources) -> Self {
        let (job_tx, job_rx) = crossbeam_channel::bounded(1);
        let (result_tx, result_rx) = crossbeam_channel::bounded(1);

        let render_resources = Arc::new(Mutex::new(render_resources));
        let render_resources_clone = render_resources.clone();

        let thread_builder = std::thread::Builder::new().name("Render Thread".to_string());
        let join_handle = thread_builder
            .spawn(
                || match Self::render_thread(job_rx, result_tx, render_resources) {
                    Ok(_) => log::info!("Render thread ended without error"),
                    Err(err) => log::info!("Render thread ended with error: {:?}", err),
                },
            )
            .unwrap();

        RenderThread {
            render_resources: render_resources_clone,
            join_handle: Some(join_handle),
            job_tx,
            result_rx,
            expecting_result: false,
        }
    }

    pub fn render(
        &mut self,
        prepared_frame: RenderFrameJob,
        presentable_frame: RafxPresentableFrame,
    ) {
        self.job_tx
            .send(RenderThreadMessage::Render(
                prepared_frame,
                presentable_frame,
            ))
            .unwrap();

        assert!(!self.expecting_result);
        self.expecting_result = true;
    }

    pub fn wait_for_render_finish(
        &mut self,
        timeout: std::time::Duration,
    ) -> Option<Result<RenderFrameJobResult, RecvTimeoutError>> {
        if self.expecting_result {
            self.expecting_result = false;
            Some(self.result_rx.recv_timeout(timeout))
        } else {
            None
        }
    }

    fn stop(&mut self) {
        self.job_tx.send(RenderThreadMessage::Finish).unwrap();
        self.join_handle.take().unwrap().join().unwrap();
    }

    fn render_thread(
        job_rx: Receiver<RenderThreadMessage>,
        result_tx: Sender<RenderFrameJobResult>,
        render_resources: Arc<Mutex<RenderResources>>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        loop {
            profiling::register_thread!();

            match job_rx.recv()? {
                RenderThreadMessage::Render(prepared_frame, frame_in_flight) => {
                    #[cfg(feature = "profile-with-tracy")]
                    profiling::tracy_client::start_noncontinuous_frame!("Render Frame");

                    log::trace!("kick off render");
                    let resource_lock = render_resources.lock().unwrap();
                    let result = prepared_frame.render_async(frame_in_flight, &*resource_lock);
                    result_tx.send(result).unwrap();

                    #[cfg(feature = "profile-with-tracy")]
                    profiling::tracy_client::finish_continuous_frame!("Render Frame");
                }
                RenderThreadMessage::Finish => {
                    log::trace!("finishing render thread");
                    break Ok(());
                }
            }
        }
    }
}

impl Drop for RenderThread {
    fn drop(&mut self) {
        self.stop();
    }
}
