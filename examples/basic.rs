use futures::stream::StreamExt;
use winit::{window::WindowBuilder, event_loop::EventLoop};
use winit_executor::EventLoopExecutor;



fn main() {
    let event_loop = EventLoop::new();

    let _window = WindowBuilder::new().build(&event_loop).unwrap();

    let event_executor = EventLoopExecutor::from(event_loop);

    event_executor.run_with(async {
        let mut events = winit_executor::window_event_producer();
        while let Some(e) = events.next().await {
            println!("{:?}", e);
        }
    })
}
