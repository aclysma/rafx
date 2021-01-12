use crate::game_renderer::RenderFrameJob;
use crossbeam_channel::{Receiver, Sender};
use rafx::api::RafxPresentableFrame;
use std::thread::JoinHandle;

enum RenderThreadMessage {
    Render(RenderFrameJob, RafxPresentableFrame),
    Finish,
}

pub struct RenderThread {
    join_handle: Option<JoinHandle<()>>,
    job_tx: Sender<RenderThreadMessage>,
}

impl RenderThread {
    pub fn start() -> Self {
        let (job_tx, job_rx) = crossbeam_channel::bounded(1);

        let thread_builder = std::thread::Builder::new().name("Render Thread".to_string());
        let join_handle = thread_builder
            .spawn(|| match Self::render_thread(job_rx) {
                Ok(_) => log::info!("Render thread ended without error"),
                Err(err) => log::info!("Render thread ended with error: {:?}", err),
            })
            .unwrap();

        RenderThread {
            join_handle: Some(join_handle),
            job_tx,
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
    }

    fn stop(&mut self) {
        self.job_tx.send(RenderThreadMessage::Finish).unwrap();
        self.join_handle.take().unwrap().join().unwrap();
    }

    fn render_thread(
        job_rx: Receiver<RenderThreadMessage>
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        loop {
            profiling::register_thread!();

            match job_rx.recv()? {
                RenderThreadMessage::Render(prepared_frame, frame_in_flight) => {
                    #[cfg(feature = "profile-with-tracy")]
                    tracy_client::start_noncontinuous_frame!("Render Frame");

                    log::trace!("kick off render");
                    prepared_frame.render_async(frame_in_flight);

                    #[cfg(feature = "profile-with-tracy")]
                    tracy_client::finish_continuous_frame!("Render Frame");
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
