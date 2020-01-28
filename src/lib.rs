use winit::event_loop::{ControlFlow, EventLoop};

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub mod event_producer;

use event_producer::EVENT_CHANNEL;

pub struct EventLoopExecutor(EventLoop<()>);

impl From<EventLoop<()>> for EventLoopExecutor {
    fn from(event_loop: EventLoop<()>) -> Self {
        EventLoopExecutor(event_loop)
    }
}

impl std::ops::Deref for EventLoopExecutor {
    type Target = EventLoop<()>;
    fn deref(&self) -> &EventLoop<()> {
        &self.0
    }
}

impl EventLoopExecutor {
    pub fn new() -> Self {
        Self::from(EventLoop::new())
    }

    pub fn run_with(self, mut fut: impl Future + 'static) -> ! {
        let waker = futures::task::noop_waker();
        self.0.run(move |event, _, control_flow| {
            let mut ctx = Context::from_waker(&waker);
            let fut = unsafe { Pin::new_unchecked(&mut fut) };

            if let Some(event) = event.to_static() {
                EVENT_CHANNEL.send(event);
            }

            match fut.poll(&mut ctx) {
                Poll::Pending => {
                    *control_flow = ControlFlow::Wait;
                }
                Poll::Ready(_) => *control_flow = ControlFlow::Exit,
            }
        })
    }
}

pub async fn spawn<T>(mut fut: impl Future<Output = T>) -> T {
    unimplemented!()
}
