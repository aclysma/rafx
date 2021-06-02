use super::render_frame_job::RenderFrameJobResult;
use super::RenderFrameJob;
use crossbeam_channel::{Receiver, Sender};
use rafx_api::RafxPresentableFrame;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

enum RenderThreadMessage {
    Render(RenderFrameJob, RafxPresentableFrame),
    Finish,
}

pub struct RenderThread {
    join_handle: Option<JoinHandle<()>>,
    job_tx: Sender<RenderThreadMessage>,

    result_rx: Receiver<RenderFrameJobResult>,
    expecting_result: AtomicBool,
}

impl RenderThread {
    pub fn start() -> Self {
        let (job_tx, job_rx) = crossbeam_channel::bounded(1);
        let (result_tx, result_rx) = crossbeam_channel::bounded(1);

        let thread_builder = std::thread::Builder::new().name("Render Thread".to_string());
        let join_handle = thread_builder
            .spawn(|| match Self::render_thread(job_rx, result_tx) {
                Ok(_) => log::info!("Render thread ended without error"),
                Err(err) => log::info!("Render thread ended with error: {:?}", err),
            })
            .unwrap();

        RenderThread {
            join_handle: Some(join_handle),
            job_tx,
            result_rx,
            expecting_result: AtomicBool::new(false),
        }
    }

    pub fn render(
        &self,
        prepared_frame: RenderFrameJob,
        presentable_frame: RafxPresentableFrame,
    ) {
        self.job_tx
            .send(RenderThreadMessage::Render(
                prepared_frame,
                presentable_frame,
            ))
            .unwrap();

        let was_expecting_result = self.expecting_result.swap(true, Ordering::Relaxed);
        assert!(!was_expecting_result);
    }

    pub fn wait_for_render_finish(&self) -> Option<RenderFrameJobResult> {
        if self.expecting_result.load(Ordering::Relaxed) {
            let was_expecting_result = self.expecting_result.swap(false, Ordering::Relaxed);
            assert!(was_expecting_result);
            Some(self.result_rx.recv().unwrap())
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
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        loop {
            profiling::register_thread!();

            match job_rx.recv()? {
                RenderThreadMessage::Render(prepared_frame, frame_in_flight) => {
                    profiling::scope!("Render Frame");

                    log::trace!("kick off render");
                    let result = prepared_frame.render_async(frame_in_flight);
                    result_tx.send(result).unwrap();
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
