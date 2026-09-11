use super::bindings::Bindings;
use crate::command::{Button, Command, Heading, JumpTarget, NavigationMode, Presentation, Speed};
use crate::config::{AppearanceConfig, Config, NavigationConfig};
use crate::core::{AppState, Direction, Glide, Mode, MotionEngine, Vector2D, navigation::Navigation};
use crate::desktop::{Appearance, Desktop, Rect};
use crate::platform::{self, CursorActuator, InputListener, InputOptions, KeyEvent, MouseButton};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};
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
  navigation_config: NavigationConfig,
  appearance: AppearanceConfig,
  navigation: Option<Navigation>,
  selected: Option<Rect>,
  held_buttons: Vec<Button>,
  held_keys: HashMap<String, Heading>,
  base_speed: Mode,
  speed_overrides: Vec<(Direction, Mode)>,
  glide: Option<(Glide, Instant)>,
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
    let navigation_config = config.navigation.clone();
    let appearance = config.appearance.clone();
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
      navigation_config,
      appearance,
      navigation: None,
      selected: None,
      held_buttons: Vec::new(),
      held_keys: HashMap::new(),
      base_speed: Mode::Normal,
      speed_overrides: vec![],
      glide: None,
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
        self
          .config
          .navigation
          .alphabets()
          .iter()
          .flat_map(|alphabet| alphabet.chars().map(|character| character.to_string()))
          .collect::<BTreeSet<_>>()
          .into_iter()
          .collect()
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

  fn desktop_mut(&mut self) -> Result<&mut Desktop> {
    self
      .desktop
      .as_mut()
      .context("visual navigation currently requires macOS")
  }

  fn activate(&mut self, mode: NavigationMode) -> Result<()> {
    if !platform::accessibility_trusted(true) {
      bail!("Enable Accessibility for Kact in System Settings > Privacy & Security, then retry");
    }
    let mut navigation_config = match mode {
      NavigationMode::Grid => self.config.navigation.for_grid(),
      NavigationMode::Elements => self.config.navigation.for_elements(),
      NavigationMode::Freestyle => self.config.navigation.clone(),
    };
    let mut alphabet = self.bindings.alphabet_for(&navigation_config.alphabet)?;
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
    let mut navigation = match mode {
      NavigationMode::Grid => Some(Navigation::grid(
        &screens,
        navigation_config.rows,
        navigation_config.columns,
        &alphabet,
      )?),
      NavigationMode::Elements => {
        let rectangles = self.desktop_mut()?.elements().unwrap_or_else(|error| {
          tracing::warn!(%error, "Element discovery unavailable; using grid");
          vec![]
        });
        if rectangles.is_empty() {
          actual_mode = NavigationMode::Grid;
          tracing::info!("No accessible targets; using grid navigation");
          None
        } else {
          Some(Navigation::from_rects(&rectangles, &alphabet)?)
        }
      }
      NavigationMode::Freestyle => None,
    };
    if actual_mode == NavigationMode::Grid && mode != NavigationMode::Grid {
      navigation_config = self.config.navigation.for_grid();
      alphabet = self.bindings.alphabet_for(&navigation_config.alphabet)?;
      navigation = Some(Navigation::grid(
        &screens,
        navigation_config.rows,
        navigation_config.columns,
        &alphabet,
      )?);
    }
    if actual_mode == NavigationMode::Elements && self.desktop()?.focus_token().ok() != token {
      bail!("focused window changed during discovery; activate elements again");
    }
    self.ensure_listener(true)?;
    self.state.input.active_directions.clear();
    self.state.velocity = Vector2D::zero();
    self.held_keys.clear();
    self.speed_overrides.clear();
    self.apply_speed();
    self.navigation = navigation;
    self.screen_layout = screens;
    self.mode = Some(actual_mode);
    self.navigation_config = navigation_config;
    self.appearance = match actual_mode {
      NavigationMode::Grid => self.config.appearance.for_grid(),
      NavigationMode::Elements => self.config.appearance.for_elements(),
      NavigationMode::Freestyle => self.config.appearance.clone(),
    };
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
    if let Err(error) = self.render() {
      self.deactivate()?;
      return Err(error);
    }
    Ok(())
  }

  fn render(&mut self) -> Result<()> {
    if let Some(desktop) = self.desktop.as_mut() {
      if let Some(navigation) = &self.navigation {
        let a = &self.appearance;
        let appearance = Appearance {
          font_size: a.font_size,
          foreground: a.foreground.clone(),
          background: a.background.clone(),
          highlight: a.highlight.clone(),
          opacity: a.opacity,
          grid_lines: a.grid_lines,
          label_position: a.label_position,
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

  fn apply_speed(&mut self) {
    self
      .state
      .input
      .set_mode(self.speed_overrides.last().map_or(self.base_speed, |(_, mode)| *mode));
  }

  fn stop_motion(&mut self) -> Result<()> {
    self.glide = None;
    self.state.input.active_directions.clear();
    self.state.velocity = Vector2D::zero();
    self.held_keys.clear();
    self.speed_overrides.clear();
    self.apply_speed();
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
    }
    self.stop_motion()
  }

  pub fn execute(&mut self, command: Command) -> Result<Value> {
    command.validate().map_err(anyhow::Error::msg)?;
    let mouse_action = matches!(
      command,
      Command::Move { .. }
        | Command::MoveTo { .. }
        | Command::Click { .. }
        | Command::ButtonDown { .. }
        | Command::ButtonUp { .. }
        | Command::Scroll { .. }
        | Command::Jump { .. }
    );
    let result = self.apply(command);
    if mouse_action && result.is_err() {
      let _ = self.deactivate();
    }
    result
  }

  fn apply(&mut self, command: Command) -> Result<Value> {
    match command {
      Command::Status => {
        return Ok(
          json!({ "running": true, "position": self.cursor.get_position().ok().map(|point| json!({"x": point.x, "y": point.y})), "active": self.mode.is_some(), "moving": !self.state.input.active_directions.is_empty(), "gliding": self.glide.is_some(), "mode": self.mode, "prefix": self.navigation.as_ref().map(|n| n.prefix.as_str()), "targets": self.navigation.as_ref().map_or(0, |n| n.targets.len()), "config": self.config_path, "global_shortcuts": self.bindings.global.len(), "navigation_keyboard": self.config.keybindings.navigation_enabled }),
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
      Command::Move { dx, dy, glide } => {
        if glide {
          let start = self.cursor.get_position()?;
          self.start_glide(start, start.add(&Vector2D::new(dx, dy)));
        } else {
          self.glide = None;
          self.cursor.move_relative(Vector2D::new(dx, dy))?;
        }
      }
      Command::MoveTo { x, y, glide } => {
        let target = Vector2D::new(x, y);
        if glide {
          self.start_glide(self.cursor.get_position()?, target);
        } else {
          self.glide = None;
          self.cursor.move_absolute(target)?;
        }
      }
      Command::MoveStart { direction, speed } => {
        // Command motion does not implicitly install a keyboard hook.
        self.glide = None;
        self.state.active = true;
        let direction = to_direction(direction);
        if let Some(speed) = speed {
          self.speed_overrides.retain(|(held, _)| *held != direction);
          self.speed_overrides.push((direction, to_mode(speed)));
          self.apply_speed();
        }
        self.state.input.press_direction(direction);
      }
      Command::MoveStop { direction } => {
        let direction = to_direction(direction);
        self.state.input.release_direction(direction);
        self.speed_overrides.retain(|(held, _)| *held != direction);
        self.apply_speed();
        if self.state.input.active_directions.is_empty() {
          self.state.velocity = Vector2D::zero();
          self.state.active = self.mode.is_some();
        }
      }
      Command::Speed { mode } => {
        self.base_speed = to_mode(mode);
        self.apply_speed();
      }
      Command::Click {
        button,
        count,
        modifiers,
      } => {
        self.glide = None;
        if self.held_buttons.contains(&button) {
          self.cursor.button_up(to_button(button), &modifiers)?;
          self.held_buttons.retain(|b| *b != button);
        } else {
          self.cursor.click(to_button(button), count, &modifiers)?;
        }
        self.deactivate()?;
      }
      Command::ButtonDown { button, modifiers } => {
        self.glide = None;
        self.cursor.button_down(to_button(button), &modifiers)?;
        if !self.held_buttons.contains(&button) {
          self.held_buttons.push(button);
        }
      }
      Command::ButtonUp { button, modifiers } => {
        self.glide = None;
        self.cursor.button_up(to_button(button), &modifiers)?;
        self.held_buttons.retain(|b| *b != button);
      }
      Command::Scroll { dx, dy } => {
        self.glide = None;
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
          &self.bindings.alphabet_for(&self.navigation_config.alphabet)?,
        )?);
        self.selected = None;
        self.render()?;
      }
      Command::Refresh => {
        if self.mode != Some(NavigationMode::Elements) {
          bail!("refresh requires element navigation");
        }
        self.activate(NavigationMode::Elements)?;
      }
      Command::Show { setting } => {
        match setting {
          Presentation::GridLines => self.appearance.grid_lines = !self.appearance.grid_lines,
          Presentation::Labels => self.labels_visible = !self.labels_visible,
          Presentation::MoreContrast => self.appearance.opacity = (self.appearance.opacity + 0.1).min(1.0),
          Presentation::LessContrast => self.appearance.opacity = (self.appearance.opacity - 0.1).max(0.1),
          Presentation::Larger | Presentation::Smaller => {
            if setting == Presentation::Larger {
              self.navigation_config.rows = self.navigation_config.rows.saturating_sub(1).max(1);
              self.navigation_config.columns = self.navigation_config.columns.saturating_sub(1).max(1);
            } else {
              self.navigation_config.rows = (self.navigation_config.rows + 1).min(100);
              self.navigation_config.columns = (self.navigation_config.columns + 1).min(100);
            }
            if self.mode == Some(NavigationMode::Grid) {
              self.navigation = Some(Navigation::grid(
                &self.screen_layout,
                self.navigation_config.rows,
                self.navigation_config.columns,
                &self.bindings.alphabet_for(&self.navigation_config.alphabet)?,
              )?);
              self.selected = None;
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
      if self.mode == Some(NavigationMode::Elements) {
        let current = self.desktop_mut()?.elements()?;
        if !current.contains(&rect) {
          self.navigation = Some(Navigation::from_rects(
            &current,
            &self.bindings.alphabet_for(&self.navigation_config.alphabet)?,
          )?);
          self.render()?;
          bail!("targets changed; select a label from the refreshed overlay");
        }
      }
      self
        .cursor
        .move_absolute(Vector2D::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0))?;
      self.glide = None;
      self.selected = Some(rect);
    }
    self.navigation = Some(navigation);
    if let Some(bounds) = selected.filter(|_| self.navigation_config.auto_click) {
      if self.mode == Some(NavigationMode::Elements) && self.desktop()?.press_element(bounds) {
        self.deactivate()?;
      } else {
        self.execute(Command::Click {
          button: Button::Left,
          count: 1,
          modifiers: vec![],
        })?;
      }
    } else {
      self.render()?;
    }
    Ok(())
  }

  fn jump(&mut self, target: JumpTarget) -> Result<()> {
    self.glide = None;
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
      if let Command::MoveStart { direction, .. } = command {
        self.held_keys.insert(event.key, direction);
      }
      self.execute(command)?;
    } else if self.navigation.is_some() && event.modifiers.is_empty() && event.key.len() == 1 && !event.repeat {
      self.select(&event.key)?;
    }
    Ok(())
  }

  pub fn sleep_duration(&self) -> Duration {
    if self.glide.is_some() || !self.state.input.active_directions.is_empty() || self.state.velocity.magnitude() > 0.01
    {
      Duration::from_secs_f64(1.0 / self.config.motion.target_fps as f64)
        .saturating_sub(self.last_tick.elapsed())
        .clamp(Duration::from_millis(1), Duration::from_millis(16))
    } else {
      Duration::from_millis(16)
    }
  }

  pub fn poll(&mut self) -> Result<()> {
    if let Some(desktop) = self.desktop.as_mut() {
      desktop.pump();
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
      self.advance_glide(now)?;
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

  fn start_glide(&mut self, start: Vector2D, target: Vector2D) {
    self.glide = None;
    if start != target {
      self.glide = Some((
        Glide::new(
          start,
          target,
          Duration::from_millis(self.config.glide.duration_ms.into()),
          self.config.glide.easing,
        ),
        Instant::now(),
      ));
    }
  }

  fn advance_glide(&mut self, now: Instant) -> Result<()> {
    let Some((glide, started)) = &self.glide else {
      return Ok(());
    };
    let (point, complete) = glide.point_at(now.duration_since(*started));
    self.cursor.move_absolute(point)?;
    if complete {
      self.glide = None;
    }
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
fn to_mode(speed: Speed) -> Mode {
  match speed {
    Speed::Normal => Mode::Normal,
    Speed::Precise => Mode::Precise,
    Speed::Fast => Mode::Fast,
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
    let navigation_config = config.navigation.clone();
    let appearance = config.appearance.clone();
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
      navigation_config,
      appearance,
      navigation: None,
      selected: None,
      held_buttons: vec![],
      held_keys: HashMap::new(),
      base_speed: Mode::Normal,
      speed_overrides: vec![],
      glide: None,
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
        speed: None,
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
  fn held_speed_overrides_restore_the_selected_mode() {
    let (mut runtime, _) = runtime();
    runtime.execute(Command::Speed { mode: Speed::Precise }).unwrap();
    runtime
      .execute(Command::MoveStart {
        direction: Heading::Right,
        speed: Some(Speed::Fast),
      })
      .unwrap();
    assert_eq!(runtime.state.input.mode, Mode::Fast);
    runtime
      .execute(Command::MoveStart {
        direction: Heading::Up,
        speed: Some(Speed::Normal),
      })
      .unwrap();
    assert_eq!(runtime.state.input.mode, Mode::Normal);
    runtime.execute(Command::MoveStop { direction: Heading::Up }).unwrap();
    assert_eq!(runtime.state.input.mode, Mode::Fast);
    runtime
      .execute(Command::MoveStop {
        direction: Heading::Right,
      })
      .unwrap();
    assert_eq!(runtime.state.input.mode, Mode::Precise);
  }
  #[test]
  fn glides_are_exact_and_pointer_actions_cancel_them() {
    let (mut runtime, output) = runtime();
    runtime
      .execute(Command::MoveTo {
        x: 100.0,
        y: 50.0,
        glide: true,
      })
      .unwrap();
    let started = runtime.glide.as_ref().unwrap().1;
    runtime.advance_glide(started + Duration::from_millis(140)).unwrap();
    assert_eq!(output.lock().unwrap().position, Vector2D::new(100.0, 50.0));
    assert!(runtime.glide.is_none());

    runtime
      .execute(Command::Move {
        dx: 100.0,
        dy: 0.0,
        glide: true,
      })
      .unwrap();
    assert!(runtime.glide.is_some());
    runtime
      .execute(Command::Move {
        dx: 10.0,
        dy: 0.0,
        glide: false,
      })
      .unwrap();
    assert!(runtime.glide.is_none());
    assert_eq!(output.lock().unwrap().position, Vector2D::new(110.0, 50.0));

    runtime
      .execute(Command::MoveTo {
        x: 300.0,
        y: 50.0,
        glide: true,
      })
      .unwrap();
    runtime
      .execute(Command::MoveStart {
        direction: Heading::Right,
        speed: None,
      })
      .unwrap();
    assert!(runtime.glide.is_none());
    runtime.execute(Command::Stop).unwrap();

    grid(&mut runtime);
    runtime
      .execute(Command::MoveTo {
        x: 300.0,
        y: 50.0,
        glide: true,
      })
      .unwrap();
    runtime.execute(Command::Select { label: "ab".into() }).unwrap();
    assert!(runtime.glide.is_none());
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
