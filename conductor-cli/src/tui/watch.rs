use anyhow::Result;
use conductor::{containers::LogOutput, Component as _, System};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::{Stream, StreamExt, TryStreamExt};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    text::Spans,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{io, pin::Pin};
use tokio::sync::{mpsc, Notify};
use tracing::{debug, trace};

#[derive(Debug)]
enum AppEvent {
    Quit,
    Next,
    Previous,
    PageUp,
    PageDown,
}

pub struct WatchApp {
    // The main window (aka top level widget).
    window: WatchWindow,
    system: System,
    events: mpsc::Receiver<AppEvent>,
}

impl WatchApp {
    pub fn new(system: System) -> WatchApp {
        let (sender, receiver) = mpsc::channel(5);
        std::thread::spawn(|| Self::input_handler_thread(sender));
        WatchApp {
            window: WatchWindow::new(),
            system,
            events: receiver,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        // setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        loop {
            terminal.draw(|f| self.render(f))?;

            let event = self.update().await?;

            match event {
                Some(AppEvent::Quit) => break,
                Some(AppEvent::Next) => self.next_machine(),
                Some(AppEvent::Previous) => self.previous_machine(),
                Some(AppEvent::PageUp) => self.scroll_log(-10),
                Some(AppEvent::PageDown) => self.scroll_log(10),
                None => (),
            }
        }

        // restore terminal
        // TODO: catch panics in application above? update: can't. Maybe do this outside the tokio
        // runtime so I can catch external panics? That requires quite a bit of rearchitecting of
        // all the commands.
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        Ok(())
    }

    fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
        self.window.render(f)
    }

    async fn update(&mut self) -> Result<Option<AppEvent>> {
        tokio::select! {
            res = self.window.update(&self.system) => {
                res?;
                Ok(None)
            },
            event = self.events.recv() => Ok(event),
        }
    }

    fn input_handler_thread(sender: mpsc::Sender<AppEvent>) -> ! {
        loop {
            let event = if let Event::Key(key) = event::read().expect("send event to main thread") {
                trace!("event: {key:?}");
                match key.code {
                    KeyCode::Char('q') => Some(AppEvent::Quit),
                    KeyCode::Esc => Some(AppEvent::Quit),
                    KeyCode::PageDown => Some(AppEvent::PageDown),
                    KeyCode::PageUp => Some(AppEvent::PageUp),
                    KeyCode::Down => Some(AppEvent::Next),
                    KeyCode::Up => Some(AppEvent::Previous),
                    _ => None,
                }
            } else {
                None
            };

            if let Some(event) = event {
                sender.blocking_send(event).expect("send event")
            }
        }
    }

    fn next_machine(&mut self) {
        self.window.next_machine();
    }

    fn previous_machine(&mut self) {
        self.window.previous_machine();
    }

    fn scroll_log(&mut self, distance: i16) {
        self.window.scroll_log(distance);
    }
}

struct WatchWindow {
    machine_list: MachineList,
    log: MachineLog,
}

impl WatchWindow {
    fn new() -> WatchWindow {
        WatchWindow {
            machine_list: MachineList::new(),
            log: MachineLog::new(None),
        }
    }

    fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
            .split(f.size());

        f.render_stateful_widget(
            self.machine_list.to_widget(),
            chunks[0],
            &mut self.machine_list.state.clone(),
        );

        self.log.render(f, chunks[1]);
    }

    async fn update(&mut self, system: &System) -> Result<()> {
        tokio::select! {
            res = self.machine_list.update(system) => res?,
            res = self.log.update(system) => res?
        }

        Ok(())
    }

    fn next_machine(&mut self) {
        if let Some(machine_name) = self.machine_list.next_machine() {
            self.log = MachineLog::new(Some(machine_name.to_string()));
        }
    }

    fn previous_machine(&mut self) {
        if let Some(machine_name) = self.machine_list.previous_machine() {
            self.log = MachineLog::new(Some(machine_name.to_string()));
        }
    }

    fn scroll_log(&mut self, distance: i16) {
        self.log.scroll(distance);
    }
}

struct MachineList {
    machines: Vec<String>,
    state: ListState,
    refresh: Notify,
    needs_init: bool,
}

impl MachineList {
    fn new() -> MachineList {
        MachineList {
            machines: Vec::new(),
            state: ListState::default(),
            refresh: Notify::new(),
            needs_init: true,
        }
    }

    fn to_widget(&self) -> List {
        let block = Block::default().title(" Machines ").borders(Borders::ALL);
        let machines_list_items: Vec<_> = self
            .machines
            .iter()
            .map(|machine| ListItem::new(machine.as_str()))
            .collect();
        List::new(machines_list_items)
            .block(block)
            .highlight_symbol(">>")
    }

    async fn update(&mut self, system: &System) -> Result<()> {
        if self.needs_init {
            self.needs_init = false;
        } else {
            self.refresh.notified().await;
        }

        for machine in &system.config().machines {
            self.machines.push(machine.name().to_string());
        }

        Ok(())
    }

    fn next_machine(&mut self) -> Option<&str> {
        let selection = self.state.selected().map(|s| s + 1).unwrap_or(0);
        if selection >= self.machines.len() {
            let count = self.machines.len();
            if count == 0 {
                None
            } else {
                Some(self.machines[count - 1].as_str())
            }
        } else {
            self.state.select(Some(selection));
            Some(self.machines[selection].as_str())
        }
    }

    fn previous_machine(&mut self) -> Option<&str> {
        let selection = self
            .state
            .selected()
            .map(|s| s.saturating_sub(1))
            .unwrap_or(0);

        if self.machines.is_empty() {
            None
        } else {
            self.state.select(Some(selection));
            Some(self.machines[selection].as_str())
        }
    }
}

struct MachineLog {
    name: Option<String>,
    log: Vec<String>,
    log_lines: usize,
    log_stream: Option<Pin<Box<dyn Stream<Item = Result<LogOutput>> + Send>>>,
    refresh: Notify,
    log_size: Option<Rect>,
    scroll: Option<u16>,
}

impl MachineLog {
    fn new(name: Option<String>) -> MachineLog {
        MachineLog {
            name,
            log: Vec::new(),
            log_lines: 0,
            log_stream: None,
            refresh: Notify::new(),
            log_size: None,
            scroll: None,
        }
    }

    fn to_widget(&self) -> Paragraph {
        let text: Vec<Spans> = self
            .log
            .iter()
            .map(|line| Spans::from(line.as_str()))
            .collect();
        Paragraph::new(text)
            .block(Block::default().title("Machine Log").borders(Borders::ALL))
            .wrap(Wrap { trim: true })
    }

    fn render<B: Backend>(&mut self, f: &mut Frame<B>, r: Rect) {
        if self.log_size.is_none() || self.log_size != Some(r) {
            self.log_size = Some(r);

            // TODO: re-compute log_lines
            let mut lines = 0;
            for line in &self.log {
                lines += count_lines(line, r.width);
            }
            self.log_lines = lines;
        }

        let offset = self
            .log_lines
            .saturating_sub(r.height.into())
            .try_into()
            .unwrap_or(u16::MAX);
        let mut scroll = self.scroll.unwrap_or(offset);
        // limit scroll to (wrapped) log lines & enable auto scroll if scrolled to/past end
        if scroll >= offset {
            self.scroll = None;
            scroll = offset;
        }
        let widget = self.to_widget();
        let scrolled_widget = widget.scroll((scroll, 0));
        f.render_widget(scrolled_widget, r);
    }

    async fn update(&mut self, system: &System) -> Result<()> {
        let Some(ref name) = self.name else {
            // No container selected, keep log_empty
            self.refresh.notified().await;

            unreachable!();
        };

        let log_stream = if let Some(log_stream) = &mut self.log_stream {
            log_stream
        } else {
            let container = system
                .containers()
                .into_iter()
                // TODO: something better than checking if the full name ends with the machine name
                .find(|&c| c.name().map(|n| n.ends_with(name)).unwrap_or(false))
                .expect("find named container");
            let log_stream = container.attach().await?;
            self.log_stream
                .insert(Box::pin(log_stream.output.map_err(|e| e.into())))
        };
        if let Some(log_item) = log_stream.next().await {
            let log_item = log_item?;
            let log_message = match log_item {
                LogOutput::StdIn { message } => message,
                LogOutput::StdOut { message } => message,
                LogOutput::StdErr { message } => message,
                LogOutput::Console { message } => message,
            };
            let log_line = String::from_utf8_lossy(&log_message).into_owned();
            if let Some(rect) = self.log_size {
                // if this isn't set, `log_lines` will be "re"-computed in `render` anyway
                self.log_lines += count_lines(&log_line, rect.width);
            }
            self.log.push(log_line);
        } else {
            // block forever
            debug!("container log stream ended");
            self.refresh.notified().await;
        }
        Ok(())
    }

    fn scroll(&mut self, distance: i16) {
        let scroll = if let Some(scroll) = self.scroll {
            scroll
        } else {
            self.log_lines
                .try_into()
                .map(|l: u16| l.saturating_sub(self.log_size.map(|r| r.height).unwrap_or(0)))
                .unwrap_or(0)
        }
        .saturating_add_signed(distance);
        self.scroll = Some(scroll);
    }
}

// TODO: this assumes that the text is '1 byte' == '1 character', this is going to cause weird
// intermitent issues with scrolling for things that aren't regular printble ASCII.
// TODO: check for inline newlines
// TODO: don't count non-printable characters
fn count_lines(s: &str, line_len: u16) -> usize {
    let line_len = line_len as usize;
    let log_len = s.len();

    // poor man's `div_ceil`
    if log_len == line_len {
        s.len() / line_len
    } else {
        (s.len() / line_len) + 1
    }
}
