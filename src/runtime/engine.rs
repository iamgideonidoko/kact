use super::bindings::Bindings;
use crate::command::{Button, Command, Heading, JumpTarget, NavigationMode, Presentation, Speed};
use crate::config::Config;
use crate::core::{AppState, Direction, Mode, MotionEngine, Vector2D, navigation::Navigation};
use crate::desktop::{Appearance, Desktop, DesktopAction, Rect};
use crate::platform::{self, CursorActuator, InputListener, InputOptions, KeyEvent, MouseButton};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

pub struct Runtime {
  pub config: Config,
  config_path: PathBuf,
  bindings: Bindings,
  state: AppState,
  engine: MotionEngine,
  cursor: Box<dyn CursorActuator>,
  desktop: Option<Desktop>,
  listener: Option<Box<dyn InputListener>>,
  active: Arc<AtomicBool>,
  labels_active: Arc<AtomicBool>,
  mode: Option<NavigationMode>,
  navigation: Option<Navigation>,
  selected: Option<Rect>,
  held_buttons: Vec<Button>,
  held_keys: HashMap<String, Heading>,
  last_tick: Instant,
  last_screen_check: Instant,
  screen_layout: Vec<Rect>,
  focus_token: Option<String>,
  refresh_at: Option<Instant>,
  cycle: usize,
  labels_visible: bool,
  pub quitting: bool,
}

impl Runtime {
  pub fn new(config: Config, config_path: PathBuf) -> Result<Self> {
    config.validate()?;
    let bindings = Bindings::new(&config)?;
    bindings.alphabet(&config)?;
    let cursor = platform::create_cursor_actuator()?;
    #[cfg(target_os = "macos")]
    let desktop = Some(Desktop::new()?);
    #[cfg(not(target_os = "macos"))]
    let desktop = None;
    let mut runtime = Self {
      engine: MotionEngine::with_modes(config.motion.clone(), config.modes.clone()),
      config,
      config_path,
      bindings,
      cursor,
      desktop,
      state: AppState::new(),
      listener: None,
      active: Arc::new(AtomicBool::new(false)),
      labels_active: Arc::new(AtomicBool::new(false)),
      mode: None,
      navigation: None,
      selected: None,
      held_buttons: Vec::new(),
      held_keys: HashMap::new(),
      last_tick: Instant::now(),
      last_screen_check: Instant::now(),
      screen_layout: vec![],
      focus_token: None,
      refresh_at: None,
      cycle: 0,
      labels_visible: true,
      quitting: false,
    };
    runtime.ensure_listener(false)?;
    Ok(runtime)
  }

  fn ensure_listener(&mut self, navigation: bool) -> Result<()> {
    if self.listener.is_some() {
      return Ok(());
    }
    if self.bindings.global.is_empty()
      && (!navigation || !self.config.keybindings.navigation_enabled && self.bindings.local.is_empty())
    {
      return Ok(());
    }
    let options = InputOptions {
      global_shortcuts: self.bindings.global.keys().cloned().collect(),
      navigation_keys: self.bindings.local.keys().cloned().collect(),
      label_keys: if self.config.keybindings.navigation_enabled {
        self.config.navigation.alphabet.chars().map(|c| c.to_string()).collect()
      } else {
        vec![]
      },
      labels_active: Arc::clone(&self.labels_active),
      active: Arc::clone(&self.active),
    };
    let mut listener = platform::create_input_listener(options)?;
    listener.start()?;
    self.listener = Some(listener);
    Ok(())
  }

  fn desktop(&self) -> Result<&Desktop> {
    self
      .desktop
      .as_ref()
      .context("visual navigation currently requires macOS")
  }

  fn activate(&mut self, mode: NavigationMode) -> Result<()> {
    if !platform::accessibility_trusted(true) {
      bail!("Enable Accessibility for Kact in System Settings > Privacy & Security, then retry");
    }
    let alphabet = self.bindings.alphabet(&self.config)?;
    let screens = if mode != NavigationMode::Freestyle {
      self.desktop()?.screens()?
    } else {
      vec![]
    };
    let mut actual_mode = mode;
    let token = if mode == NavigationMode::Elements {
      self.desktop()?.focus_token().ok()
    } else {
      None
    };
    let navigation = match mode {
      NavigationMode::Grid => Some(Navigation::grid(
        &screens,
        self.config.navigation.rows,
        self.config.navigation.columns,
        &alphabet,
      )?),
      NavigationMode::Elements => {
        let rectangles = self.desktop()?.elements().unwrap_or_else(|error| {
          tracing::warn!(%error, "Element discovery unavailable; using grid");
          vec![]
        });
        if rectangles.is_empty() {
          actual_mode = NavigationMode::Grid;
          tracing::info!("No accessible targets; using grid navigation");
          Some(Navigation::grid(
            &screens,
            self.config.navigation.rows,
            self.config.navigation.columns,
            &alphabet,
          )?)
        } else {
          Some(Navigation::from_rects(&rectangles, &alphabet)?)
        }
      }
      NavigationMode::Freestyle => None,
    };
    if actual_mode == NavigationMode::Elements && self.desktop()?.focus_token().ok() != token {
      bail!("focused window changed during discovery; activate elements again");
    }
    self.ensure_listener(true)?;
    self.state.input.active_directions.clear();
    self.state.velocity = Vector2D::zero();
    self.held_keys.clear();
    self.navigation = navigation;
    self.screen_layout = screens;
    self.mode = Some(actual_mode);
    self.focus_token = if actual_mode == NavigationMode::Elements {
      token
    } else {
      None
    };
    self.refresh_at = None;
    self.selected = None;
    self.state.active = true;
    self.labels_active.store(self.navigation.is_some(), Ordering::Release);
    self.active.store(true, Ordering::Release);
    self.last_tick = Instant::now();
    if let Some(desktop) = self.desktop.as_mut() {
      desktop.set_active(true);
    }
    if let Err(error) = self.render() {
      self.deactivate()?;
      return Err(error);
    }
    Ok(())
  }

  fn render(&mut self) -> Result<()> {
    if let Some(desktop) = self.desktop.as_mut() {
      if let Some(navigation) = &self.navigation {
        let a = &self.config.appearance;
        let appearance = Appearance {
          font_size: a.font_size,
          foreground: a.foreground.clone(),
          background: a.background.clone(),
          highlight: a.highlight.clone(),
          opacity: a.opacity,
          grid_lines: a.grid_lines,
        };
        let mut targets = navigation.visible();
        if !self.labels_visible {
          for target in &mut targets {
            target.label.clear();
          }
        }
        desktop.show(&targets, &navigation.prefix, &appearance)?;
      } else {
        desktop.hide();
      }
    }
    Ok(())
  }

  fn stop_motion(&mut self) -> Result<()> {
    self.state.input.active_directions.clear();
    self.state.velocity = Vector2D::zero();
    self.held_keys.clear();
    self.cursor.release_all()?;
    self.held_buttons.clear();
    Ok(())
  }

  fn deactivate(&mut self) -> Result<()> {
    self.active.store(false, Ordering::Release);
    self.labels_active.store(false, Ordering::Release);
    self.state.active = false;
    self.mode = None;
    self.focus_token = None;
    self.refresh_at = None;
    self.navigation = None;
    self.selected = None;
    if let Some(desktop) = self.desktop.as_mut() {
      desktop.hide();
      desktop.set_active(false);
    }
    self.stop_motion()
  }

  pub fn execute(&mut self, command: Command) -> Result<Value> {
    command.validate().map_err(anyhow::Error::msg)?;
    match command {
      Command::Status => {
        return Ok(
          json!({ "running": true, "active": self.mode.is_some(), "moving": !self.state.input.active_directions.is_empty(), "mode": self.mode, "prefix": self.navigation.as_ref().map(|n| n.prefix.as_str()), "targets": self.navigation.as_ref().map_or(0, |n| n.targets.len()), "config": self.config_path, "global_shortcuts": self.bindings.global.len(), "navigation_keyboard": self.config.keybindings.navigation_enabled }),
        );
      }
      Command::Activate { mode } => self.activate(mode)?,
      Command::Toggle { mode } => {
        if self.mode.is_some() {
          self.deactivate()?;
        } else {
          self.activate(mode)?;
        }
      }
      Command::Deactivate => self.deactivate()?,
      Command::Stop => self.stop_motion()?,
      Command::Quit => {
        self.deactivate()?;
        self.quitting = true;
      }
      Command::Cancel => {
        if let Some(navigation) = self.navigation.as_mut().filter(|n| !n.prefix.is_empty()) {
          navigation.clear_prefix();
          self.render()?;
        } else {
          self.deactivate()?;
        }
      }
      Command::Move { dx, dy } => self.cursor.move_relative(Vector2D::new(dx, dy))?,
      Command::MoveTo { x, y } => self.cursor.move_absolute(Vector2D::new(x, y))?,
      Command::MoveStart { direction } => {
        // Command motion does not implicitly install a keyboard hook.
        self.state.active = true;
        self.state.input.press_direction(to_direction(direction));
      }
      Command::MoveStop { direction } => {
        self.state.input.release_direction(to_direction(direction));
        if self.state.input.active_directions.is_empty() {
          self.state.velocity = Vector2D::zero();
          self.state.active = self.mode.is_some();
        }
      }
      Command::Speed { mode } => {
        self.state.input.mode = match mode {
          Speed::Normal => Mode::Normal,
          Speed::Precise => Mode::Precise,
          Speed::Fast => Mode::Fast,
        }
      }
      Command::Click {
        button,
        count,
        modifiers,
      } => {
        if self.held_buttons.contains(&button) {
          self.cursor.button_up(to_button(button), &modifiers)?;
          self.held_buttons.retain(|b| *b != button);
        } else {
          self.cursor.click(to_button(button), count, &modifiers)?;
        }
        self.deactivate()?;
      }
      Command::ButtonDown { button, modifiers } => {
        self.cursor.button_down(to_button(button), &modifiers)?;
        if !self.held_buttons.contains(&button) {
          self.held_buttons.push(button);
        }
      }
      Command::ButtonUp { button, modifiers } => {
        self.cursor.button_up(to_button(button), &modifiers)?;
        self.held_buttons.retain(|b| *b != button);
      }
      Command::Scroll { dx, dy } => {
        self.cursor.scroll(dx, dy)?;
        if self.mode == Some(NavigationMode::Elements) {
          self.refresh_at = Some(Instant::now() + Duration::from_millis(180));
          if let Some(desktop) = self.desktop.as_mut() {
            desktop.hide();
          }
        }
      }
      Command::Jump { target } => self.jump(target)?,
      Command::Select { label } => self.select(&label)?,
      Command::Backspace => {
        self
          .navigation
          .as_mut()
          .context("no active target navigation")?
          .backspace();
        self.render()?;
      }
      Command::Refine => {
        let rectangle = self.selected.context("select a grid cell before refining")?;
        if self.mode != Some(NavigationMode::Grid) {
          bail!("refinement requires grid mode");
        }
        self.navigation = Some(Navigation::grid(
          &[rectangle],
          3,
          3,
          &self.bindings.alphabet(&self.config)?,
        )?);
        self.selected = None;
        self.render()?;
      }
      Command::Show { setting } => {
        match setting {
          Presentation::GridLines => self.config.appearance.grid_lines = !self.config.appearance.grid_lines,
          Presentation::Labels => self.labels_visible = !self.labels_visible,
          Presentation::MoreContrast => {
            self.config.appearance.opacity = (self.config.appearance.opacity + 0.1).min(1.0)
          }
          Presentation::LessContrast => {
            self.config.appearance.opacity = (self.config.appearance.opacity - 0.1).max(0.1)
          }
          Presentation::Larger | Presentation::Smaller => {
            if setting == Presentation::Larger {
              self.config.navigation.rows = self.config.navigation.rows.saturating_sub(1).max(1);
              self.config.navigation.columns = self.config.navigation.columns.saturating_sub(1).max(1);
            } else {
              self.config.navigation.rows = (self.config.navigation.rows + 1).min(100);
              self.config.navigation.columns = (self.config.navigation.columns + 1).min(100);
            }
            if self.mode == Some(NavigationMode::Grid) {
              self.activate(NavigationMode::Grid)?;
            }
          }
        }
        self.render()?;
      }
      Command::Reload => {
        self.update_config(Config::load(&self.config_path)?)?;
      }
      _ => bail!("this command must run locally, not through the service"),
    }
    Ok(json!({ "ok": true }))
  }

  fn select(&mut self, label: &str) -> Result<()> {
    if self.refresh_at.is_some() {
      bail!("targets are refreshing after scrolling");
    }
    if self.mode == Some(NavigationMode::Elements) && self.desktop()?.focus_token().ok() != self.focus_token {
      self.deactivate()?;
      bail!("focused window changed; activate elements again");
    }
    let mut navigation = self
      .navigation
      .clone()
      .context("activate grid or elements before selecting a label")?;
    let mut selected = None;
    for (index, character) in label.chars().enumerate() {
      let previous = navigation.prefix.clone();
      let target = navigation.type_char(character);
      if navigation.prefix == previous {
        bail!("no target matches `{previous}{character}`");
      }
      if let Some(target) = target {
        if index + 1 != label.chars().count() {
          bail!("label contains characters after a complete target");
        }
        selected = Some(target.bounds);
        navigation.clear_prefix();
      }
    }
    if let Some(rect) = selected {
      self
        .cursor
        .move_absolute(Vector2D::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0))?;
      self.selected = Some(rect);
    }
    self.navigation = Some(navigation);
    if selected.is_some() && self.config.navigation.auto_click {
      self.execute(Command::Click {
        button: Button::Left,
        count: 1,
        modifiers: vec![],
      })?;
    } else {
      self.render()?;
    }
    Ok(())
  }

  fn jump(&mut self, target: JumpTarget) -> Result<()> {
    let current = self.cursor.get_position()?;
    let screens = self.desktop()?.screens()?;
    let rect = screens
      .iter()
      .find(|r| current.x >= r.x && current.x < r.x + r.width && current.y >= r.y && current.y < r.y + r.height)
      .or_else(|| screens.first())
      .context("no connected displays")?;
    let right = rect.x + rect.width - 1.0;
    let bottom = rect.y + rect.height - 1.0;
    let center = Vector2D::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    let position = match target {
      JumpTarget::Top => Vector2D::new(current.x, rect.y),
      JumpTarget::Bottom => Vector2D::new(current.x, bottom),
      JumpTarget::Left => Vector2D::new(rect.x, current.y),
      JumpTarget::Right => Vector2D::new(right, current.y),
      JumpTarget::Center => {
        self.cycle = 0;
        center
      }
      JumpTarget::Cycle => {
        let points = [
          center,
          Vector2D::new(rect.x, rect.y),
          Vector2D::new(right, rect.y),
          Vector2D::new(right, bottom),
          Vector2D::new(rect.x, bottom),
        ];
        let point = points[self.cycle % 5];
        self.cycle = (self.cycle + 1) % 5;
        point
      }
    };
    self.cursor.move_absolute(position)?;
    Ok(())
  }

  pub fn update_config(&mut self, config: Config) -> Result<()> {
    config.validate()?;
    let bindings = Bindings::new(&config)?;
    bindings.alphabet(&config)?;
    // Config reload exits navigation to avoid changing held-key ownership mid-session.
    self.deactivate()?;
    if let Some(mut listener) = self.listener.take() {
      listener.stop()?;
    }
    let old_config = std::mem::replace(&mut self.config, config);
    let old_bindings = std::mem::replace(&mut self.bindings, bindings);
    if let Err(error) = self.ensure_listener(false) {
      self.config = old_config;
      self.bindings = old_bindings;
      self
        .ensure_listener(false)
        .context("failed to restore previous input listener")?;
      return Err(error);
    }
    self.engine.update_config(self.config.motion.clone());
    self.engine.update_modes(self.config.modes.clone());
    Ok(())
  }

  fn key(&mut self, event: KeyEvent) -> Result<()> {
    if !event.pressed {
      if let Some(direction) = self.held_keys.remove(&event.key) {
        self.execute(Command::MoveStop { direction })?;
      }
      return Ok(());
    }
    let shortcut = event.shortcut();
    let command = self
      .bindings
      .global
      .get(&shortcut)
      .or_else(|| {
        self
          .active
          .load(Ordering::Acquire)
          .then(|| self.bindings.local.get(&shortcut))
          .flatten()
      })
      .cloned();
    if let Some(command) = command {
      if event.repeat
        && !matches!(
          command,
          Command::Move { .. } | Command::Scroll { .. } | Command::Backspace | Command::Jump { .. }
        )
      {
        return Ok(());
      }
      if let Command::MoveStart { direction } = command {
        self.held_keys.insert(event.key, direction);
      }
      self.execute(command)?;
    } else if self.navigation.is_some() && event.modifiers.is_empty() && event.key.len() == 1 && !event.repeat {
      self.select(&event.key)?;
    }
    Ok(())
  }

  pub fn poll(&mut self) -> Result<()> {
    let events = self.desktop.as_mut().map(|d| d.pump()).unwrap_or_default();
    for event in events {
      let command = match event {
        DesktopAction::ActivateGrid => Command::Activate {
          mode: NavigationMode::Grid,
        },
        DesktopAction::ActivateElements => Command::Activate {
          mode: NavigationMode::Elements,
        },
        DesktopAction::ActivateFreestyle => Command::Activate {
          mode: NavigationMode::Freestyle,
        },
        DesktopAction::Deactivate => Command::Deactivate,
        DesktopAction::Quit => Command::Quit,
        DesktopAction::OpenConfig => {
          self.open_config()?;
          continue;
        }
        DesktopAction::Help => {
          self.open_help()?;
          continue;
        }
      };
      if let Err(error) = self.execute(command) {
        tracing::error!(%error, "Menu action failed");
      }
    }
    for _ in 0..256 {
      let event = match self.listener.as_mut() {
        Some(listener) => listener.next_event(),
        None => break,
      };
      match event {
        Ok(Some(event)) => {
          if let Err(error) = self.key(event) {
            tracing::debug!(%error, "Navigation key ignored");
          }
        }
        Ok(None) => break,
        Err(error) => {
          self.deactivate()?;
          return Err(error.into());
        }
      }
    }
    let now = Instant::now();
    if self.refresh_at.is_some_and(|deadline| now >= deadline) {
      self.activate(NavigationMode::Elements)?;
    }
    let frame = Duration::from_secs_f64(1.0 / self.config.motion.target_fps as f64);
    if now.duration_since(self.last_tick) >= frame {
      let delta = now.duration_since(self.last_tick).as_secs_f64().min(0.05);
      self.last_tick = now;
      let (velocity, movement) = self.engine.tick(&self.state, delta);
      self.state.velocity = velocity;
      if movement.magnitude() > 0.0001 {
        self.cursor.move_relative(movement)?;
      }
    }
    if self.navigation.is_some() && now.duration_since(self.last_screen_check) >= Duration::from_secs(1) {
      self.last_screen_check = now;
      if self.desktop()?.screens()? != self.screen_layout
        || self.mode == Some(NavigationMode::Elements) && self.desktop()?.focus_token().ok() != self.focus_token
      {
        self.deactivate()?;
        tracing::info!("Display layout or focused window changed; navigation cancelled");
      }
    }
    Ok(())
  }

  fn open_config(&self) -> Result<()> {
    if !self.config_path.exists() {
      if let Some(parent) = self.config_path.parent() {
        std::fs::create_dir_all(parent)?;
      }
      use std::io::Write;
      let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&self.config_path)?;
      file.write_all(toml::to_string_pretty(&self.config)?.as_bytes())?;
    }
    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
      .args(["-t"])
      .arg(&self.config_path)
      .spawn()?;
    Ok(())
  }

  fn open_help(&self) -> Result<()> {
    let path = std::env::temp_dir().join(format!("kact-help-{}.txt", std::process::id()));
    std::fs::write(
      &path,
      "Kact\n\nUse `kact --help` for commands.\nActivate Grid or Elements, then type a label to move.\nArrow keys nudge; Alt+arrows move farther; Cmd+arrows jump to edges.\nEnter clicks, = begins a drag, Enter drops, \\ double-clicks.\n[ middle-clicks, ] right-clicks; Shift+arrows scroll.\nEscape clears a label or exits. Cmd+H hides.\nPreferences opens your TOML config.\n",
    )?;
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").args(["-t"]).arg(path).spawn()?;
    Ok(())
  }
}
impl Drop for Runtime {
  fn drop(&mut self) {
    let _ = self.deactivate();
    if let Some(mut listener) = self.listener.take() {
      let _ = listener.stop();
    }
  }
}
fn to_direction(direction: Heading) -> Direction {
  match direction {
    Heading::Up => Direction::Up,
    Heading::Down => Direction::Down,
    Heading::Left => Direction::Left,
    Heading::Right => Direction::Right,
  }
}
fn to_button(button: Button) -> MouseButton {
  match button {
    Button::Left => MouseButton::Left,
    Button::Right => MouseButton::Right,
    Button::Middle => MouseButton::Middle,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::Mutex;

  #[derive(Default)]
  struct Output {
    position: Vector2D,
    clicks: usize,
    downs: usize,
    ups: usize,
    releases: usize,
  }
  struct Cursor(Arc<Mutex<Output>>);
  impl CursorActuator for Cursor {
    fn move_relative(&mut self, delta: Vector2D) -> crate::Result<()> {
      let mut output = self.0.lock().unwrap();
      output.position = output.position.add(&delta);
      Ok(())
    }
    fn move_absolute(&mut self, point: Vector2D) -> crate::Result<()> {
      self.0.lock().unwrap().position = point;
      Ok(())
    }
    fn get_position(&self) -> crate::Result<Vector2D> {
      Ok(self.0.lock().unwrap().position)
    }
    fn click(&mut self, _: MouseButton, count: u8, _: &[String]) -> crate::Result<()> {
      self.0.lock().unwrap().clicks += count as usize;
      Ok(())
    }
    fn button_down(&mut self, _: MouseButton, _: &[String]) -> crate::Result<()> {
      self.0.lock().unwrap().downs += 1;
      Ok(())
    }
    fn button_up(&mut self, _: MouseButton, _: &[String]) -> crate::Result<()> {
      self.0.lock().unwrap().ups += 1;
      Ok(())
    }
    fn scroll(&mut self, _: i32, _: i32) -> crate::Result<()> {
      Ok(())
    }
    fn release_all(&mut self) -> crate::Result<()> {
      self.0.lock().unwrap().releases += 1;
      Ok(())
    }
  }
  fn runtime() -> (Runtime, Arc<Mutex<Output>>) {
    let config = Config::default();
    let output = Arc::new(Mutex::new(Output::default()));
    let runtime = Runtime {
      engine: MotionEngine::with_modes(config.motion.clone(), config.modes.clone()),
      bindings: Bindings::new(&config).unwrap(),
      config,
      config_path: PathBuf::from("unused"),
      state: AppState::new(),
      cursor: Box::new(Cursor(Arc::clone(&output))),
      desktop: None,
      listener: None,
      active: Arc::new(AtomicBool::new(false)),
      labels_active: Arc::new(AtomicBool::new(false)),
      mode: None,
      navigation: None,
      selected: None,
      held_buttons: vec![],
      held_keys: HashMap::new(),
      last_tick: Instant::now(),
      last_screen_check: Instant::now(),
      screen_layout: vec![],
      focus_token: None,
      refresh_at: None,
      cycle: 0,
      labels_visible: true,
      quitting: false,
    };
    (runtime, output)
  }
  fn grid(runtime: &mut Runtime) {
    runtime.navigation = Some(
      Navigation::grid(
        &[Rect {
          x: -200.0,
          y: 0.0,
          width: 400.0,
          height: 200.0,
        }],
        2,
        2,
        "ab",
      )
      .unwrap(),
    );
    runtime.mode = Some(NavigationMode::Grid);
    runtime.state.active = true;
    runtime.active.store(true, Ordering::Release);
  }
  #[test]
  fn selection_is_atomic_and_cancel_clears_then_exits() {
    let (mut runtime, output) = runtime();
    grid(&mut runtime);
    assert!(runtime.execute(Command::Select { label: "aax".into() }).is_err());
    assert_eq!(runtime.navigation.as_ref().unwrap().prefix, "");
    assert_eq!(output.lock().unwrap().position, Vector2D::zero());
    runtime.execute(Command::Select { label: "a".into() }).unwrap();
    runtime.execute(Command::Cancel).unwrap();
    assert!(runtime.mode.is_some());
    runtime.execute(Command::Select { label: "ab".into() }).unwrap();
    assert_eq!(output.lock().unwrap().position, Vector2D::new(100.0, 50.0));
    runtime.execute(Command::Refine).unwrap();
    assert_eq!(runtime.navigation.as_ref().unwrap().targets.len(), 9);
    runtime.execute(Command::Cancel).unwrap();
    assert!(runtime.mode.is_none());
    assert!(!runtime.active.load(Ordering::Acquire));
  }
  #[test]
  fn click_drops_drag_and_exits_without_an_extra_click() {
    let (mut runtime, output) = runtime();
    grid(&mut runtime);
    runtime
      .execute(Command::ButtonDown {
        button: Button::Left,
        modifiers: vec![],
      })
      .unwrap();
    runtime
      .execute(Command::Click {
        button: Button::Left,
        count: 1,
        modifiers: vec![],
      })
      .unwrap();
    assert_eq!(output.lock().unwrap().clicks, 0);
    assert_eq!(output.lock().unwrap().ups, 1);
    assert!(runtime.mode.is_none());
    assert!(runtime.held_buttons.is_empty());
  }
  #[test]
  fn shell_motion_does_not_install_or_enable_keyboard_capture() {
    let (mut runtime, _) = runtime();
    runtime
      .execute(Command::MoveStart {
        direction: Heading::Right,
      })
      .unwrap();
    assert!(runtime.listener.is_none());
    assert!(!runtime.active.load(Ordering::Acquire));
    let status = runtime.execute(Command::Status).unwrap();
    assert_eq!(status["active"], false);
    assert_eq!(status["moving"], true);
    runtime
      .execute(Command::MoveStop {
        direction: Heading::Right,
      })
      .unwrap();
    assert_eq!(runtime.state.velocity, Vector2D::zero());
    assert!(!runtime.state.active);
  }
  #[test]
  fn local_bindings_only_run_active_and_repeat_does_not_click() {
    let (mut runtime, output) = runtime();
    let enter = KeyEvent {
      key: "enter".into(),
      modifiers: vec![],
      pressed: true,
      repeat: false,
    };
    runtime.key(enter.clone()).unwrap();
    assert_eq!(output.lock().unwrap().clicks, 0);
    grid(&mut runtime);
    runtime
      .key(KeyEvent {
        repeat: true,
        ..enter.clone()
      })
      .unwrap();
    assert_eq!(output.lock().unwrap().clicks, 0);
    runtime.key(enter).unwrap();
    assert_eq!(output.lock().unwrap().clicks, 1);
  }
  #[test]
  fn invalid_reload_preserves_configuration_and_targets() {
    let (mut runtime, _) = runtime();
    grid(&mut runtime);
    let mut invalid = Config::default();
    invalid.motion.target_fps = 0;
    assert!(runtime.update_config(invalid).is_err());
    assert_eq!(runtime.config.motion.target_fps, 144);
    assert!(runtime.navigation.is_some());
  }
}
