#[allow(dead_code)]
use crate::events::StatefulList;
use tui::backend::Backend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::widgets::{Block, BorderType, Borders, Clear, List, Paragraph, Text};

use tui::terminal::Frame;

use crate::parser::{DepListList, DepVersion, DepVersionReq};

pub struct App {
    data: DepListList,
    items: StatefulList<String>,
    popup_shown: bool,
    style_uptodate: Style,
    style_patch: Style,
    style_minor: Style,
    style_major: Style,
}

impl App {
    pub fn new(dep_list_list: DepListList) -> App {
        let dep_names = dep_list_list.get_dep_names();
        App {
            data: dep_list_list,
            items: StatefulList::with_items(dep_names),
            popup_shown: false,
            style_uptodate: Style::default().fg(Color::White),
            style_patch: Style::default().fg(Color::Yellow),
            style_minor: Style::default().fg(Color::Magenta),
            style_major: Style::default().fg(Color::Red),
        }
    }

    pub fn show_popup(&mut self) {
        self.popup_shown = true;
    }
    pub fn hide_popup(&mut self) {
        self.popup_shown = false;
    }
    pub fn toggle_popup(&mut self) {
        self.popup_shown = !self.popup_shown;
    }

    pub fn next(&mut self) {
        self.items.next()
    }
    pub fn previous(&mut self) {
        self.items.previous()
    }
    pub fn render_version_selector<B: Backend>(&mut self, f: &mut Frame<B>) {
        if self.popup_shown {
            let block = Block::default()
                .title("Choose version")
                .borders(Borders::ALL);
            let area = centered_rect(50, 80, f.size());
            f.render_widget(Clear, area); //this clears out the background
            f.render_widget(block, area);
        }
    }
    pub fn render_dependency_info<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        let dep = self.data.get_dep(&self.items.get_item());
        if let Some(d) = dep {
            let text = [
                Text::styled("Name", Style::default().fg(Color::Red)),
                Text::raw(format!(" {}\n", d.name)),
                Text::styled("Specified Version", Style::default().fg(Color::Blue)),
                Text::raw(format!(" {}\n", &d.get_specified_version())),
                Text::styled("Current Version", Style::default().fg(Color::Blue)),
                Text::raw(format!(" {}\n", &d.get_current_version())),
                Text::styled("Upgradeable Version", Style::default().fg(Color::Blue)),
                Text::raw(format!(" {}\n", &d.get_latest_semver_version())),
                Text::styled("Latest Version", Style::default().fg(Color::Blue)),
                Text::raw(format!(" {}\n", &d.get_latest_version())),
                Text::styled("Author", Style::default().fg(Color::Green)),
                Text::raw(format!(" {}\n", &d.author.to_string())),
                Text::styled("Homepage", Style::default().fg(Color::Magenta)),
                Text::raw(format!(" {}\n", &d.homepage.to_string())),
                Text::styled("License", Style::default().fg(Color::Yellow)),
                Text::raw(format!(" {}\n", &d.license.to_string())),
                Text::styled("Description", Style::default().fg(Color::Cyan)),
                Text::raw(format!(" {}\n", &d.description.to_string())),
            ];
            let block = Paragraph::new(text.iter())
                .block(
                    Block::default()
                        .title("Info")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::White)),
                )
                .style(Style::default())
                .alignment(Alignment::Left)
                .wrap(true);
            f.render_widget(block, chunk);
        }
    }

    pub fn render_dependency_list<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        let items = self.items.items.iter().map(|i| list_format(&i));
        let block = List::new(items)
            .block(
                Block::default()
                    .title("Dependencies")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::White)),
            )
            .style(Style::default())
            .highlight_style(Style::default().bg(Color::White));
        f.render_stateful_widget(block, chunk, &mut self.items.state);
    }
}

pub fn list_format(i: &str) -> Text {
    if i == "futures" {
        Text::styled(i, Style::default().fg(Color::Green))
    } else {
        Text::raw(i)
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
