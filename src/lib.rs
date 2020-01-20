#![feature(const_fn)]

use winit::{
    event::{self, Event},
    event_loop::{ControlFlow, EventLoop},
};

use futures::stream::Stream;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crossbeam_channel::{Receiver, Sender, TryRecvError};
use once_cell::sync::Lazy;

struct StaticChannel<T> {
    sender: Sender<T>,
    receiver: Receiver<T>,
}

impl<T> StaticChannel<T> {
    const fn new() -> Lazy<Self> {
        Lazy::new(|| {
            let (sender, receiver) = crossbeam_channel::unbounded();
            Self { sender, receiver }
        })
    }

    fn send(&self, msg: T) -> () {
        self.sender.send(msg).unwrap();
    }

    fn get_recv_clone(&self) -> Receiver<T> {
        self.receiver.clone()
    }
}
static EVENT_CHANNEL: Lazy<StaticChannel<Event<'static, ()>>> = StaticChannel::new();

struct EventProducer(Receiver<Event<'static, ()>>);

pub fn event_producer() -> impl Stream<Item = Event<'static, ()>> {
    EventProducer::new()
}

impl EventProducer {
    fn new() -> Self {
        Self(EVENT_CHANNEL.get_recv_clone())
    }
}

impl Stream for EventProducer {
    type Item = Event<'static, ()>;

    fn poll_next(self: Pin<&mut Self>, _ctx: &mut Context) -> Poll<Option<Self::Item>> {
        match self.as_ref().0.try_recv() {
            Ok(event) => Poll::Ready(Some(event)),
            Err(TryRecvError::Disconnected) => Poll::Ready(None),
            _ => Poll::Pending,
        }
    }
}

#[derive(Debug)]
pub struct DeviceEvent {
    device_id: event::DeviceId,
    event: event::DeviceEvent,
}

struct DeviceEventProducer(EventProducer);

pub fn device_event_producer() -> impl Stream<Item = DeviceEvent> {
    DeviceEventProducer::new()
}

impl DeviceEventProducer {
    fn new() -> Self {
        Self(EventProducer::new())
    }
}

impl Stream for DeviceEventProducer {
    type Item = DeviceEvent;

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Self::Item>> {
        match unsafe { self.map_unchecked_mut(|this| &mut this.0) }.poll_next(ctx) {
            Poll::Ready(Some(Event::DeviceEvent { device_id, event })) => {
                Poll::Ready(Some(DeviceEvent { device_id, event }))
            }
            Poll::Ready(None) => Poll::Ready(None),
            _ => Poll::Pending,
        }
    }
}

#[derive(Debug)]
pub struct WindowEvent {
    window_id: winit::window::WindowId,
    event: event::WindowEvent<'static>,
}

pub struct WindowEventProducer(EventProducer);

pub fn window_event_producer() -> impl Stream<Item = WindowEvent> {
    WindowEventProducer::new()
}

impl WindowEventProducer {
    fn new() -> Self {
        Self(EventProducer::new())
    }
}

impl Stream for WindowEventProducer {
    type Item = WindowEvent;

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Self::Item>> {
        match unsafe { self.map_unchecked_mut(|this| &mut this.0) }.poll_next(ctx) {
            Poll::Ready(Some(Event::WindowEvent { window_id, event })) => {
                Poll::Ready(Some(WindowEvent { window_id, event }))
            }
            Poll::Ready(None) => Poll::Ready(None),
            _ => Poll::Pending,
        }
    }
}

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
