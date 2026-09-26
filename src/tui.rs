//! `gutcheck -i`: score once, then explore. Move the threshold, sort, filter and re-ask live.
use crate::input::Record;
use crate::model::Model;
use crate::packs;
use anyhow::Result;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::crossterm::execute;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::{Frame, Terminal};
use std::io::{stderr, Stderr};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

enum Cmd {
    Ask(u64, Vec<String>),
}

enum Msg {
    Score(u64, usize, f32),
    Done(u64, usize),
    Failed(String),
}

/// Scores every text for the latest question; a newer question interrupts the run in progress.
fn worker(mut model: Model, texts: Vec<String>, all: bool, cmds: Receiver<Cmd>, out: Sender<Msg>) {
    let mut next = cmds.recv().ok();
    while let Some(Cmd::Ask(gen, questions)) = next.take() {
        let qs = match questions.iter().map(|q| model.yes_no(q)).collect::<Result<Vec<_>>>() {
            Ok(qs) => qs,
            Err(e) => {
                let _ = out.send(Msg::Failed(format!("{e:#}")));
                next = cmds.recv().ok();
                continue;
            }
        };
        for (i, text) in texts.iter().enumerate() {
            if let Ok(newer) = cmds.try_recv() {
                next = Some(newer);
                break;
            }
            let p = if text.trim().is_empty() {
                0.0
            } else {
                let mut ps = vec![];
                for (k, q) in qs.iter().enumerate() {
                    match model.probs(gen as usize * 64 + k, q, text) {
                        Ok(p) => ps.push(p[1]),
                        Err(e) => {
                            let _ = out.send(Msg::Failed(format!("{e:#}")));
                            return;
                        }
                    }
                }
                if all { ps.iter().cloned().fold(f32::MAX, f32::min) } else { ps.iter().cloned().fold(f32::MIN, f32::max) }
            };
            if out.send(Msg::Score(gen, i, p)).is_err() {
                return;
            }
        }
        if next.is_none() {
            let _ = out.send(Msg::Done(gen, model.calls));
            next = cmds.recv().ok();
        }
    }
}

struct App {
    records: Vec<Record>,
    scores: Vec<Option<f32>>,
    gen: u64,
    questions: Vec<String>,
    editing: Option<String>,
    threshold: f32,
    by_score: bool,
    only_matches: bool,
    list: ListState,
    scored: usize,
    done: Option<usize>, // model calls, once the run finished
    error: Option<String>,
    started: Instant,
}

impl App {
    /// Record indices in display order.
    fn view(&self) -> Vec<usize> {
        let mut v: Vec<usize> = (0..self.records.len()).filter(|&i| !self.only_matches || self.scores[i].is_some_and(|p| p >= self.threshold)).collect();
        if self.by_score {
            v.sort_by(|&a, &b| self.scores[b].unwrap_or(-1.0).total_cmp(&self.scores[a].unwrap_or(-1.0)));
        }
        v
    }

    fn matches(&self) -> Vec<usize> {
        self.view().into_iter().filter(|&i| self.scores[i].is_some_and(|p| p >= self.threshold)).collect()
    }
}

fn heat(p: f32) -> Style {
    let c = if p >= 0.75 { Color::Red } else if p >= 0.5 { Color::Yellow } else { Color::DarkGray };
    Style::default().fg(c).add_modifier(if p >= 0.5 { Modifier::BOLD } else { Modifier::empty() })
}

fn draw(f: &mut Frame, app: &mut App) {
    let [head, body, foot] = Layout::vertical([Constraint::Length(3), Constraint::Min(3), Constraint::Length(2)]).areas(f.area());
    let dim = Style::default().fg(Color::DarkGray);

    let question = match &app.editing {
        Some(text) => Line::from(vec![Span::styled("❯ ", Style::default().fg(Color::Magenta)), Span::raw(text.clone()), Span::styled("▏", Style::default().fg(Color::Magenta))]),
        None => Line::from(vec![Span::styled("❯ ", Style::default().fg(Color::Magenta)), Span::styled(app.questions.join("  |  "), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
    };
    let title = if app.editing.is_some() { " new question · enter to ask · esc to cancel " } else { " gutcheck " };
    f.render_widget(Paragraph::new(question).block(Block::default().borders(Borders::ALL).border_style(dim).title(title)), head);

    let view = app.view();
    let width = body.width.saturating_sub(16) as usize;
    let items: Vec<ListItem> = view.iter().map(|&i| {
        let p = app.scores[i];
        let (score, bar) = match p {
            Some(p) => {
                let n = (p.clamp(0.0, 1.0) * 8.0).round() as usize;
                (format!("{p:.2}"), (format!("{}", "━".repeat(n)), format!("{}", "─".repeat(8 - n))))
            }
            None => ("....".into(), (String::new(), "─".repeat(8))),
        };
        let style = p.map_or(dim, heat);
        let text: String = app.records[i].show.lines().next().unwrap_or("").chars().take(width).collect();
        ListItem::new(Line::from(vec![Span::styled(score, style), Span::raw(" "), Span::styled(bar.0, style), Span::styled(bar.1, dim), Span::raw("  "), Span::raw(text)]))
    }).collect();
    match app.list.selected() {
        None if !items.is_empty() => app.list.select(Some(0)),
        Some(s) if s >= items.len() => app.list.select(items.len().checked_sub(1)),
        _ => {}
    }
    f.render_stateful_widget(List::new(items).highlight_style(Style::default().add_modifier(Modifier::REVERSED)), body, &mut app.list);

    let total = app.records.len();
    let progress = match app.done {
        Some(calls) => format!("done · {calls} model calls · {:.1} s", app.started.elapsed().as_secs_f32()),
        None => format!("scoring {}/{total}", app.scored),
    };
    let status = format!(" {} shown · threshold {:.2} · {progress}", app.matches().len(), app.threshold);
    let keys = " ←/→ threshold   / new question   s sort   f filter   enter print matches   q quit";
    let err = app.error.as_deref().map(|e| Span::styled(format!("  {e}"), Style::default().fg(Color::Red))).unwrap_or_default();
    f.render_widget(Paragraph::new(vec![Line::from(vec![Span::styled(status, Style::default().fg(Color::Cyan)), err]), Line::styled(keys, dim)]), foot);
}

struct Restore(Terminal<CrosstermBackend<Stderr>>);

impl Drop for Restore {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.0.backend_mut(), LeaveAlternateScreen);
    }
}

/// Runs the explorer. Returns the matches at the final threshold when the user presses enter, None if they quit.
pub fn run(model: Model, records: Vec<Record>, questions: Vec<String>, all: bool, threshold: f32) -> Result<Option<Vec<Record>>> {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (msg_tx, msg_rx) = mpsc::channel();
    let texts: Vec<String> = records.iter().map(|r| r.judge.clone()).collect();
    std::thread::spawn(move || worker(model, texts, all, cmd_rx, msg_tx));
    cmd_tx.send(Cmd::Ask(1, questions.clone()))?;

    enable_raw_mode()?;
    execute!(stderr(), EnterAlternateScreen)?;
    let mut term = Restore(Terminal::new(CrosstermBackend::new(stderr()))?);
    let n = records.len();
    let mut app = App { records, scores: vec![None; n], gen: 1, questions, editing: None, threshold, by_score: true, only_matches: true, list: ListState::default(), scored: 0, done: None, error: None, started: Instant::now() };

    loop {
        while let Ok(msg) = msg_rx.try_recv() {
            match msg {
                Msg::Score(g, i, p) if g == app.gen => {
                    app.scores[i] = Some(p);
                    app.scored += 1;
                }
                Msg::Done(g, calls) if g == app.gen => app.done = Some(calls),
                Msg::Failed(e) => app.error = Some(e),
                _ => {}
            }
        }
        term.0.draw(|f| draw(f, &mut app))?;
        if !event::poll(Duration::from_millis(60))? {
            continue;
        }
        let Event::Key(key) = event::read()? else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Ok(None);
        }
        if let Some(text) = app.editing.as_mut() {
            match key.code {
                KeyCode::Esc => app.editing = None,
                KeyCode::Backspace => { text.pop(); }
                KeyCode::Char(c) => text.push(c),
                KeyCode::Enter => {
                    let asked = std::mem::take(text);
                    app.editing = None;
                    match packs::resolve(asked.trim()) {
                        Ok(q) if !q.is_empty() => {
                            app.questions = vec![q.clone()];
                            app.gen += 1;
                            app.scores = vec![None; n];
                            (app.scored, app.done, app.error) = (0, None, None);
                            app.started = Instant::now();
                            cmd_tx.send(Cmd::Ask(app.gen, vec![q]))?;
                        }
                        Ok(_) => {}
                        Err(e) => app.error = Some(e.to_string()),
                    }
                }
                _ => {}
            }
            continue;
        }
        let len = app.view().len();
        let cur = app.list.selected().unwrap_or(0);
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
            KeyCode::Enter => return Ok(Some(app.matches().into_iter().map(|i| app.records[i].clone()).collect())),
            KeyCode::Down | KeyCode::Char('j') => app.list.select(Some((cur + 1).min(len.saturating_sub(1)))),
            KeyCode::Up | KeyCode::Char('k') => app.list.select(Some(cur.saturating_sub(1))),
            KeyCode::PageDown => app.list.select(Some((cur + 10).min(len.saturating_sub(1)))),
            KeyCode::PageUp => app.list.select(Some(cur.saturating_sub(10))),
            KeyCode::Char('g') | KeyCode::Home => app.list.select(Some(0)),
            KeyCode::Char('G') | KeyCode::End => app.list.select(Some(len.saturating_sub(1))),
            KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('l') => app.threshold = (app.threshold + 0.05).min(1.0),
            KeyCode::Left | KeyCode::Char('-') | KeyCode::Char('h') => app.threshold = (app.threshold - 0.05).max(0.0),
            KeyCode::Char('s') => app.by_score = !app.by_score,
            KeyCode::Char('f') => app.only_matches = !app.only_matches,
            KeyCode::Char('/') => app.editing = Some(String::new()),
            _ => {}
        }
    }
}
