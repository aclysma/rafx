use crossbeam_channel::{Receiver, Sender};
use crate::game_renderer::RenderFrameJob;
use std::thread::JoinHandle;

enum RenderThreadMessage {
    Render(RenderFrameJob),
    Finish,
}

pub struct RenderThread {
    join_handle: Option<JoinHandle<()>>,
    job_tx: Sender<RenderThreadMessage>,
}

impl RenderThread {
    pub fn start() -> Self {
        let (job_tx, job_rx) = crossbeam_channel::bounded(1);
        let join_handle = std::thread::spawn(|| {
            match Self::render_thread(job_rx) {
                Ok(result) => log::info!("Render thread ended without error"),
                Err(err) => log::info!("Render thread ended with error: {:?}", err)
            }
        });

        RenderThread {
            join_handle: Some(join_handle),
            job_tx,
        }
    }

    pub fn render(&mut self, prepared_frame: RenderFrameJob) {
        self.job_tx.send(RenderThreadMessage::Render(prepared_frame)).unwrap();
    }

    fn stop(&mut self) {
        self.job_tx.send(RenderThreadMessage::Finish).unwrap();
        self.join_handle.take().unwrap().join().unwrap();
    }

    fn render_thread(job_rx: Receiver<RenderThreadMessage>) -> std::result::Result<(), Box<dyn std::error::Error>> {
        loop {
            match job_rx.recv()? {
                RenderThreadMessage::Render(prepared_frame) => {
                    log::trace!("kick off render");
                    prepared_frame.render_async();
                },
                RenderThreadMessage::Finish => {
                    log::trace!("finishing render thread");
                    break Ok(())
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