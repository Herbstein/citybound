#![feature(proc_macro)]
#![allow(dead_code)]
#![feature(plugin)]
#![feature(conservative_impl_trait)]
#![plugin(clippy)]
#![allow(no_effect, unnecessary_operation)]
// Enable this for memory tracking with Instruments/MacOS
// and for much better stacktraces for memory issues
#![feature(alloc_system)]
extern crate alloc_system;

extern crate ordered_float;
extern crate itertools;
extern crate random;
extern crate fnv;

extern crate kay;
#[macro_use]
extern crate kay_macros;
extern crate monet;
extern crate descartes;

mod core;
mod game;

use monet::{Renderer, Control, AddDebugText};
use core::simulation::{Simulation, Tick};
use game::lanes_and_cars::{Lane, TransferLane};
use game::lanes_and_cars::lane_rendering::{LaneAsphalt, LaneMarker, TransferLaneMarkerGaps};
use game::lanes_and_cars::lane_thing_collector::ThingCollector;
use game::lanes_and_cars::planning::{CurrentPlan};
use kay::Individual;

const SECONDS_PER_TICK : f32 = 1.0 / 20.0;

fn main() {    
    let mut system = Box::new(kay::ActorSystem::new());
    unsafe {
        kay::THE_SYSTEM = &mut *system as *mut kay::ActorSystem;
    }

    game::setup(&mut system);
    game::setup_ui(&mut system);

    let simulatables = vec![
        system.broadcast_id::<Lane>(),
        system.broadcast_id::<TransferLane>()
    ];
    core::simulation::setup(&mut system, simulatables);

    let renderables = vec![
        system.broadcast_id::<Lane>(),
        system.broadcast_id::<TransferLane>(),
        system.individual_id::<ThingCollector<LaneAsphalt>>(),
        system.individual_id::<ThingCollector<LaneMarker>>(),
        system.individual_id::<ThingCollector<TransferLaneMarkerGaps>>(),
        system.individual_id::<CurrentPlan>(),
    ];
    let window = core::ui::setup_window_and_renderer(&mut system, renderables);

    let mut simulation_panicked : Option<String> = None;
    let mut last_frame = std::time::Instant::now();

    system.process_all_messages();

    loop {
        Renderer::id() << AddDebugText{
            scene_id: 0,
            key: "Frame".chars().collect(),
            value: format!("{:.2} ms", last_frame.elapsed().as_secs() as f32 * 1000.0 + last_frame.elapsed().subsec_nanos() as f32 / 10.0E5).as_str().chars().collect()
        };
        last_frame = std::time::Instant::now();
        if !core::ui::process_events(&window) {return}

        if let Some(error) = simulation_panicked.clone() {
            system.clear_all_clearable_messages();
            system.process_all_messages();
            Renderer::id() << AddDebugText{
                scene_id: 0,
                key: "SIMULATION PANIC".chars().collect(),
                value: error.as_str().chars().collect()
            };
        } else {
            let simulation_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                system.process_all_messages();
                
                Simulation::id() << Tick{dt: SECONDS_PER_TICK, current_tick: 0};

                system.process_all_messages();

                Renderer::id() << Control::Render;

                system.process_all_messages();
            }));
            if simulation_result.is_err() {
                system.clear_all_clearable_messages();
                let msg = match simulation_result.unwrap_err().downcast::<String>() {
                    Ok(string) => (*string),
                    Err(any) => match any.downcast::<&'static str>() {
                        Ok(static_str) => (*static_str).to_string(),
                        Err(_) => "Weird error type".to_string()
                    }
                };
                println!("Simulation Panic!\n{:?}", msg);
                simulation_panicked = Some(msg);
            }
        }

        Renderer::id() << Control::Submit;

        system.process_all_messages();
    }
}