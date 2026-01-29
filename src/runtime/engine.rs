use crate::config::Config;
use crate::core::{AppState, MotionEngine};
use crate::platform::{InputEvent, create_cursor_actuator, create_input_listener};
use crate::{Error, Result};
use crossbeam_channel::{Receiver, Sender, bounded};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub enum ControlMessage {
  UpdateConfig(Config),
  Shutdown,
}

pub struct Runtime {
  state: Arc<Mutex<AppState>>,
  control_tx: Sender<ControlMessage>,
  input_handle: Option<thread::JoinHandle<()>>,
  motion_handle: Option<thread::JoinHandle<()>>,
}

impl Runtime {
  pub fn new(config: Config) -> Result<Self> {
    let state = Arc::new(Mutex::new(AppState::new()));
    let config_arc = Arc::new(Mutex::new(config));
    let (control_tx, control_rx) = bounded::<ControlMessage>(10);
    let (event_tx, event_rx) = bounded::<InputEvent>(100);

    // Thread A: input listener (blocking, OS hooks)
    let input_handle = Self::spawn_input_thread(event_tx)?;

    // Thread B: motion engine (non-blocking, pure logic)
    let motion_handle = Self::spawn_motion_thread(Arc::clone(&state), Arc::clone(&config_arc), event_rx, control_rx)?;

    Ok(Self {
      state,
      control_tx,
      input_handle: Some(input_handle),
      motion_handle: Some(motion_handle),
    })
  }

  fn spawn_input_thread(event_tx: Sender<InputEvent>) -> Result<thread::JoinHandle<()>> {
    let handle = thread::Builder::new()
      .name("kact-input".to_string())
      .spawn(move || {
        if let Err(e) = Self::input_thread_main(event_tx) {
          tracing::error!("Input thread error: {}", e);
        }
      })
      .map_err(|e| Error::Platform(format!("Failed to spawn input thread: {}", e)))?;

    Ok(handle)
  }

  fn input_thread_main(event_tx: Sender<InputEvent>) -> Result<()> {
    let mut listener = create_input_listener()?;
    listener.start()?;

    loop {
      match listener.next_event()? {
        Some(event) => {
          if event_tx.send(event).is_err() {
            tracing::warn!("Event channel closed, stopping input listener");
            break;
          }
        }
        None => {
          thread::sleep(Duration::from_millis(1));
        }
      }
    }

    listener.stop()?;
    Ok(())
  }

  fn spawn_motion_thread(
    state: Arc<Mutex<AppState>>,
    config: Arc<Mutex<Config>>,
    event_rx: Receiver<InputEvent>,
    control_rx: Receiver<ControlMessage>,
  ) -> Result<thread::JoinHandle<()>> {
    let handle = thread::Builder::new()
      .name("kact-motion".to_string())
      .spawn(move || {
        if let Err(e) = Self::motion_thread_main(state, config, event_rx, control_rx) {
          tracing::error!("Motion thread error: {}", e);
        }
      })
      .map_err(|e| Error::Platform(format!("Failed to spawn motion thread: {}", e)))?;

    Ok(handle)
  }

  fn motion_thread_main(
    state: Arc<Mutex<AppState>>,
    config: Arc<Mutex<Config>>,
    event_rx: Receiver<InputEvent>,
    control_rx: Receiver<ControlMessage>,
  ) -> Result<()> {
    let mut actuator = create_cursor_actuator()?;
    let mut engine = {
      let cfg = config.lock().unwrap();
      MotionEngine::new(cfg.motion.clone())
    };

    let target_fps = {
      let cfg = config.lock().unwrap();
      cfg.motion.target_fps
    };

    let frame_duration = Duration::from_secs_f64(1.0 / target_fps as f64);
    let mut last_tick = Instant::now();

    loop {
      // Check for control messages (config updates, shutdown)
      if let Ok(msg) = control_rx.try_recv() {
        match msg {
          ControlMessage::UpdateConfig(new_config) => {
            tracing::info!("Hot-reloading configuration");
            engine.update_config(new_config.motion.clone());
            *config.lock().unwrap() = new_config;
          }
          ControlMessage::Shutdown => {
            tracing::info!("Motion thread shutting down");
            break;
          }
        }
      }

      // Process input events (non-blocking)
      while let Ok(event) = event_rx.try_recv() {
        Self::handle_input_event(&state, event);
      }

      // Motion tick
      let now = Instant::now();
      let delta_time = (now - last_tick).as_secs_f64();
      last_tick = now;

      let (new_velocity, delta_position) = {
        let current_state = state.lock().unwrap();
        engine.tick(&current_state, delta_time)
      };

      // Update state
      {
        let mut s = state.lock().unwrap();
        s.velocity = new_velocity;
        s.position = s.position.add(&delta_position);
      }

      // Move cursor (if there's movement)
      if delta_position.magnitude() > 0.01 {
        if let Err(e) = actuator.move_relative(delta_position) {
          tracing::error!("Failed to move cursor: {}", e);
        }
      }

      // Check emergency stop
      {
        let s = state.lock().unwrap();
        if s.emergency_stop {
          tracing::warn!("Emergency stop triggered!");
          break;
        }
      }

      // Frame rate limiting
      let elapsed = Instant::now() - now;
      if elapsed < frame_duration {
        thread::sleep(frame_duration - elapsed);
      }
    }

    Ok(())
  }

  fn handle_input_event(state: &Arc<Mutex<AppState>>, event: InputEvent) {
    let mut s = state.lock().unwrap();

    match event {
      InputEvent::DirectionPressed(dir) => {
        s.input.press_direction(dir);
      }
      InputEvent::DirectionReleased(dir) => {
        s.input.release_direction(dir);
      }
      InputEvent::ModeChanged(mode) => {
        s.input.set_mode(mode);
        tracing::info!("Mode changed to {:?}", mode);
      }
      InputEvent::ToggleActive => {
        s.toggle_active();
        tracing::info!("Active state: {}", s.active);
      }
      InputEvent::EmergencyStop => {
        s.trigger_emergency_stop();
        tracing::warn!("Emergency stop activated!");
      }
    }
  }

  pub fn update_config(&self, config: Config) -> Result<()> {
    self
      .control_tx
      .send(ControlMessage::UpdateConfig(config))
      .map_err(|_| Error::ChannelSend)?;
    Ok(())
  }

  pub fn shutdown(mut self) -> Result<()> {
    tracing::info!("Shutting down runtime");

    self
      .control_tx
      .send(ControlMessage::Shutdown)
      .map_err(|_| Error::ChannelSend)?;

    if let Some(handle) = self.motion_handle.take() {
      handle
        .join()
        .map_err(|_| Error::Platform("Failed to join motion thread".to_string()))?;
    }

    if let Some(handle) = self.input_handle.take() {
      handle
        .join()
        .map_err(|_| Error::Platform("Failed to join input thread".to_string()))?;
    }

    Ok(())
  }

  pub fn get_state(&self) -> AppState {
    self.state.lock().unwrap().clone()
  }
}
