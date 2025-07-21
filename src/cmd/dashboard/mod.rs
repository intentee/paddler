use std::io::stdout;
use std::io::Stdout;
use std::net::SocketAddr;

use anyhow::Result;
use crossterm::event::Event;
use crossterm::event::EventStream;
use crossterm::event::KeyCode;
use crossterm::event::KeyEventKind;
use crossterm::execute;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::LeaveAlternateScreen;
use futures::FutureExt;
use futures::StreamExt;
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;
use reqwest_eventsource::Event as RequestEvent;
use reqwest_eventsource::EventSource;
use tokio::runtime::Runtime;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::balancer::upstream_peer_pool::UpstreamPeerPoolInfo;
use crate::cmd::dashboard::app::App;

pub mod app;
pub mod ui;

async fn fetch_registered_agents(management_addr: SocketAddr) -> EventSource {
    EventSource::get(format!("http://{}/api/v1/agents/stream", management_addr))
}

pub async fn ratatui_main(management_addr: &SocketAddr) -> Result<()> {
    let mut terminal = ratatui::init();

    let (app_needs_to_stop_tx, mut app_needs_to_stop_rx_update) = broadcast::channel::<bool>(1);
    let (upstream_peer_pool_tx, mut upstream_peer_pool_rx) =
        mpsc::channel::<UpstreamPeerPoolInfo>(1);
    let (app_needs_to_render_app_error_tx, mut app_needs_to_render_error_rx) =
        mpsc::channel::<String>(1);

    let mut agents_stream = fetch_registered_agents(*management_addr).await;

    let update_handle: JoinHandle<Result<()>> = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = app_needs_to_stop_rx_update.recv() => {
                    break Ok(())
                },
                Some(event) = agents_stream.next() => {
                    match event {
                        Ok(RequestEvent::Open) => None,
                        Ok(RequestEvent::Message(message)) => {
                            match serde_json::from_str::<UpstreamPeerPoolInfo>(&message.data) {
                                Ok(upstream_peer_pool) => {
                                    upstream_peer_pool_tx.send(upstream_peer_pool).await.ok()
                                },
                                Err(err) => {
                                    app_needs_to_render_app_error_tx.send(format!("Error parsing response - {}", err)).await.ok();
                                    None
                                },
                            }
                        },
                        Err(err) => {
                            app_needs_to_render_app_error_tx.send(format!("Error receiving event - {}", err)).await.ok();
                            None
                        }
                    };
                }
            }
        }
    });

    let mut app_needs_to_stop_rx_render = app_needs_to_stop_tx.subscribe();

    let render_handle = tokio::spawn(async move {
        let mut app = App::new()?;
        let mut reader = EventStream::new();

        loop {
            terminal.try_draw(|frame| app.draw(frame))?;

            tokio::select! {
                _ = app_needs_to_stop_rx_render.recv() => {
                    stop_rendering(&mut terminal)?;

                    return Ok(())
                },
                Some(app_error) = app_needs_to_render_error_rx.recv() => {
                    app.error = Some(app_error);
                },
                Some(upstream_peer_pool) = upstream_peer_pool_rx.recv() => {
                    app.set_registered_agents(upstream_peer_pool)?;
                },
                Some(Ok(evt)) = reader.next().fuse() => {
                    match evt {
                        Event::Resize(_, _) => {},
                        Event::Key(key) => {
                            if key.kind == KeyEventKind::Press {
                                match key.code {
                                    KeyCode::Char('q') | KeyCode::Esc => {
                                        app_needs_to_stop_tx.send(true).ok();
                                    }
                                    KeyCode::Char('j') | KeyCode::Down => app.next_row(),
                                    KeyCode::Char('k') | KeyCode::Up => app.previous_row(),
                                    _ => {}
                                }
                            }
                        },
                        _ => {}
                    }
                }
            }
        }
    });

    let join = tokio::try_join!(update_handle, render_handle)?;

    ratatui::restore();

    join.1
}

fn stop_rendering(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    terminal.clear()?;
    let mut stdout = stdout();
    execute!(stdout, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    terminal.show_cursor()?;

    Ok(())
}

pub fn handle(management_addr: &SocketAddr) -> Result<()> {
    Runtime::new()?.block_on(ratatui_main(management_addr))?;
    Ok(())
}
