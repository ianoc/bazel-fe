use crate::app::App;
use std::time::Duration;

use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, Spans},
    widgets::canvas::{Canvas, Line, Map, MapResolution, Rectangle},
    widgets::{
        Axis, BarChart, Block, Borders, Chart, Dataset, Gauge, List, ListItem, Paragraph, Row,
        Sparkline, Table, Tabs, Wrap,GraphType
    },
    Frame,
};

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(f.size());
    let titles = app
        .tabs
        .titles
        .iter()
        .map(|t| Spans::from(Span::styled(*t, Style::default().fg(Color::Green))))
        .collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(app.title))
        .highlight_style(Style::default().fg(Color::Yellow))
        .select(app.tabs.index);
    f.render_widget(tabs, chunks[0]);
    match app.tabs.index {
        0 => draw_first_tab(f, app, chunks[1]),
        1 => draw_second_tab(f, app, chunks[1]),
        _ => {}
    };
}

fn draw_first_tab<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let chunks = Layout::default()
        .constraints(
            [
                Constraint::Length(15),
                Constraint::Min(7),
                Constraint::Length(7),
            ]
            .as_ref(),
        )
        .split(area);
    draw_top_bar(f, app, chunks[0]);
    // draw_gauges(f, app, chunks[0]);
    draw_charts(f, app, chunks[1]);
    draw_text(f, app, chunks[2]);
}

fn draw_top_bar<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let chunks = Layout::default()
        .constraints(
            vec![
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(33),
            ]
            .as_ref(),
        )
        .direction(Direction::Horizontal)
        .split(area);

    let bar_gap = if (chunks[0].width > 50) { 2 } else { 1 };
    let bar_width = (chunks[0].width - bar_gap - 5) / (app.nemonics.len()) as u16;
    let active_nemonics = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::RIGHT)
                .title("Active Action Types:"),
        )
        .data(&app.nemonics)
        .bar_width(bar_width)
        .bar_gap(bar_gap)
        .bar_set(if app.enhanced_graphics {
            symbols::bar::NINE_LEVELS
        } else {
            symbols::bar::THREE_LEVELS
        })
        .value_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Blue)
                .add_modifier(Modifier::ITALIC),
        )
        .label_style(Style::default().fg(Color::Yellow))
        .bar_style(Style::default().fg(Color::Blue));
    f.render_widget(active_nemonics, chunks[0]);

    let sub_chunks = Layout::default()
        .constraints(
            vec![
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ]
            .as_ref(),
        )
        .direction(Direction::Vertical)
        .split(chunks[1]);

    let bar_gap = if (sub_chunks[0].width > 50) { 2 } else { 1 };
    let bar_width = (sub_chunks[0].width - bar_gap - 5) / (app.completed_actions.len()) as u16;
    let completed_actions = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::RIGHT)
                .title("Launched Actions:"),
        )
        .data(&app.completed_actions)
        .bar_width(bar_width)
        .bar_gap(bar_gap)
        .bar_set(if app.enhanced_graphics {
            symbols::bar::NINE_LEVELS
        } else {
            symbols::bar::THREE_LEVELS
        })
        .value_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::ITALIC),
        )
        .label_style(Style::default().fg(Color::Yellow))
        .bar_style(Style::default().fg(Color::Green));
    f.render_widget(completed_actions, sub_chunks[0]);
    let bar_gap = if (sub_chunks[1].width > 50) { 2 } else { 1 };
    let bar_width = (sub_chunks[1].width - bar_gap - 5) / (app.completed_actions.len()) as u16;
    let completed_actions = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::RIGHT)
                .title("Completed Actions:"),
        )
        .data(&app.completed_actions)
        .bar_width(bar_width)
        .bar_gap(bar_gap)
        .bar_set(if app.enhanced_graphics {
            symbols::bar::NINE_LEVELS
        } else {
            symbols::bar::THREE_LEVELS
        })
        .value_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::ITALIC),
        )
        .label_style(Style::default().fg(Color::Yellow))
        .bar_style(Style::default().fg(Color::Green));
    f.render_widget(completed_actions, sub_chunks[1]);


    let data: Vec<(f64, f64)> = app.sparkline.points.iter().enumerate().map (|(k, v)| (k as f64, *v as f64 % 20.0)).collect();

    let datasets = vec![
        Dataset::default()
            .graph_type(GraphType::Line)
            .marker(
                symbols::Marker::Braille
            )
            .style(Style::default().fg(Color::Yellow))
            .data(&data),
    ];

    let chart = Chart::new(datasets)
    .block(
        Block::default()
            .title(Span::styled(
                "Pending Actions",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ))
    )
         .x_axis(Axis::default()
         .title(Span::styled("X Axis", Style::default().fg(Color::Red)))
         .style(Style::default().fg(Color::White))
         .bounds([0.0, data.len() as f64])
         .labels(["0.0", "5.0", "10.0"].iter().cloned().map(Span::from).collect()))
    .y_axis(
        Axis::default()
            .style(Style::default().fg(Color::Gray))
            .bounds([0.0, 20.0])
            .labels(vec![
                Span::styled("0", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled("20", Style::default().add_modifier(Modifier::BOLD)),
            ]),
    );
f.render_widget(chart, chunks[2]);

    // let max_sparkline_v = app.sparkline.points.iter().max().map(|e| *e).unwrap_or_default();
    // let sparkline = Sparkline::default()
    //     .block(Block::default().title(format!("Pending Actions: (max: {})", max_sparkline_v)))
    //     .style(Style::default().fg(Color::Green))
    //     .data(&app.sparkline.points)
    //     .max(32)
    //     .bar_set(if app.enhanced_graphics {
    //         symbols::bar::NINE_LEVELS
    //     } else {
    //         symbols::bar::THREE_LEVELS
    //     });
    // f.render_widget(sparkline, chunks[2]);
}

fn draw_gauges<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let chunks = Layout::default()
        .constraints([Constraint::Length(2), Constraint::Length(3)].as_ref())
        .margin(1)
        .split(area);
    let block = Block::default().borders(Borders::ALL).title("Graphs");
    f.render_widget(block, area);

    let label = format!("{:.2}%", app.progress * 100.0);
    let gauge = Gauge::default()
        .block(Block::default().title("Gauge:"))
        .gauge_style(
            Style::default()
                .fg(Color::Magenta)
                .bg(Color::Black)
                .add_modifier(Modifier::ITALIC | Modifier::BOLD),
        )
        .label(label)
        .ratio(app.progress);
    f.render_widget(gauge, chunks[0]);

    let sparkline = Sparkline::default()
        .block(Block::default().title("Sparkline:"))
        .style(Style::default().fg(Color::Green))
        .data(&app.sparkline.points)
        .bar_set(if app.enhanced_graphics {
            symbols::bar::NINE_LEVELS
        } else {
            symbols::bar::THREE_LEVELS
        });
    f.render_widget(sparkline, chunks[1]);
}

fn draw_charts<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
  
}

fn draw_text<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let action_style = Style::default().fg(Color::Blue);
    let target_style = Style::default().fg(Color::Yellow);
    let test_style = Style::default().fg(Color::Magenta);
    let unknown_style = Style::default().fg(Color::Red);
    let logs: Vec<ListItem> = app
        .action_logs
        .items
        .iter()
        .map(|&(level, evt, run_time)| {
            let s = match level {
                "ACTION" => action_style,
                "TARGET" => target_style,
                "TEST" => test_style,
                _ => unknown_style,
            };
            let content = vec![Spans::from(vec![
                Span::styled(format!("{:<9}", level), s),
                Span::raw(evt),
                Span::raw(format!(" in {:?}", Duration::from_secs(run_time.into()))),
            ])];
            ListItem::new(content)
        })
        .collect();
    let logs = List::new(logs).block(Block::default().borders(Borders::ALL).title("Completion events"));
    f.render_stateful_widget(logs, area, &mut app.action_logs.state);
}

fn draw_second_tab<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let chunks = Layout::default()
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .direction(Direction::Horizontal)
        .split(area);
    let up_style = Style::default().fg(Color::Green);
    let failure_style = Style::default()
        .fg(Color::Red)
        .add_modifier(Modifier::RAPID_BLINK | Modifier::CROSSED_OUT);
    let header = ["Server", "Location", "Status"];
    let rows = app.servers.iter().map(|s| {
        let style = if s.status == "Up" {
            up_style
        } else {
            failure_style
        };
        Row::StyledData(vec![s.name, s.location, s.status].into_iter(), style)
    });
    let table = Table::new(header.iter(), rows)
        .block(Block::default().title("Servers").borders(Borders::ALL))
        .header_style(Style::default().fg(Color::Yellow))
        .widths(&[
            Constraint::Length(15),
            Constraint::Length(15),
            Constraint::Length(10),
        ]);
    f.render_widget(table, chunks[0]);

    let map = Canvas::default()
        .block(Block::default().title("World").borders(Borders::ALL))
        .paint(|ctx| {
            ctx.draw(&Map {
                color: Color::White,
                resolution: MapResolution::High,
            });
            ctx.layer();
            ctx.draw(&Rectangle {
                x: 0.0,
                y: 30.0,
                width: 10.0,
                height: 10.0,
                color: Color::Yellow,
            });
            for (i, s1) in app.servers.iter().enumerate() {
                for s2 in &app.servers[i + 1..] {
                    ctx.draw(&Line {
                        x1: s1.coords.1,
                        y1: s1.coords.0,
                        y2: s2.coords.0,
                        x2: s2.coords.1,
                        color: Color::Yellow,
                    });
                }
            }
            for server in &app.servers {
                let color = if server.status == "Up" {
                    Color::Green
                } else {
                    Color::Red
                };
                ctx.print(server.coords.1, server.coords.0, "X", color);
            }
        })
        .marker(if app.enhanced_graphics {
            symbols::Marker::Braille
        } else {
            symbols::Marker::Dot
        })
        .x_bounds([-180.0, 180.0])
        .y_bounds([-90.0, 90.0]);
    f.render_widget(map, chunks[1]);
}
