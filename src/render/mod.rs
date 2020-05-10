#[allow(dead_code)]
use crate::events::{StatefulList, TabsState};
use tui::backend::Backend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::widgets::{Block, BorderType, Borders, Clear, List, Paragraph, Tabs, Text};

use std::process::Command;
use tui::terminal::Frame;

use crate::parser::{Dep, DepListList, DepVersion, DepVersionReq};

pub struct App {
    data: DepListList,
    items: StatefulList<String>,
    versions: StatefulList<String>,
    tabs: TabsState,
    popup_shown: bool,
    help_menu_shown: bool,
    style_uptodate: Style,
    style_patch: Style,
    style_minor: Style,
    style_major: Style,
}

impl App {
    pub fn new(dep_list_list: DepListList) -> App {
        let dep_kinds = dep_list_list.get_dep_kinds();
        let dep_names = dep_list_list.get_dep_names_of_kind(&dep_kinds[0]);
        let mut dep_versions = vec![];
        if let Some(dep) = dep_list_list.get_dep(&dep_names[0]) {
            dep_versions = dep.get_version_strings();
        }
        App {
            data: dep_list_list,
            items: StatefulList::with_items(dep_names),
            versions: StatefulList::with_items(dep_versions),
            tabs: TabsState::new(dep_kinds),
            popup_shown: false,
            help_menu_shown: false,
            style_uptodate: Style::default().fg(Color::White),
            style_patch: Style::default().fg(Color::Yellow),
            style_minor: Style::default().fg(Color::Magenta),
            style_major: Style::default().fg(Color::Red),
        }
    }

    pub fn get_current_dep(&mut self) -> Option<Dep> {
        self.data.get_dep(&self.items.get_item())
    }

    pub fn open_homepage(&mut self) {
        let dep = self.data.get_dep(&self.items.get_item());
        if let Some(d) = dep {
            let homepage = d.homepage;

            Command::new("open")
                .arg(homepage)
                .output()
                .expect("Failed to execute command");
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
    pub fn toggle_help_menu(&mut self) {
        self.help_menu_shown = !self.help_menu_shown;
    }

    pub fn tab_next(&mut self) {
        self.tabs.next();
        let dep_names = self
            .data
            .get_dep_names_of_kind(&self.tabs.titles[self.tabs.index]);
        self.items = StatefulList::with_items(dep_names);
        self.items.next();
    }
    pub fn tab_previous(&mut self) {
        self.tabs.previous();
        let dep_names = self
            .data
            .get_dep_names_of_kind(&self.tabs.titles[self.tabs.index]);
        self.items = StatefulList::with_items(dep_names);
        self.items.next();
    }
    pub fn next(&mut self) {
        if self.popup_shown {
            self.versions.next();
        } else {
            self.items.next();
            let mut dep_versions = vec![];
            if let Some(dep) = self.get_current_dep() {
                dep_versions = dep.get_version_strings();
            }
            self.versions = StatefulList::with_items(dep_versions);
            self.versions.next();
        }
    }
    pub fn previous(&mut self) {
        if self.popup_shown {
            self.versions.previous();
        } else {
            self.items.previous();
            let mut dep_versions = vec![];
            if let Some(dep) = self.get_current_dep() {
                dep_versions = dep.get_version_strings();
            }
            self.versions = StatefulList::with_items(dep_versions);
            self.versions.next();
        }
    }
    pub fn render_help_menu<B: Backend>(&mut self, f: &mut Frame<B>) {
        if self.help_menu_shown {
            let help_items = [
                ["?", "show help menu"],
                ["j/down", "move down"],
                ["k/up", "move up"],
                ["h/left", "prev tab"],
                ["l/right", "next tab"],
                ["v/space", "show version list"],
                ["o", "open homepage"],
            ];
            let mut text = vec![];
            for item in help_items.iter() {
                text.push(Text::styled(
                    format!("{:<10}", item[0]),
                    Style::default().fg(Color::Green),
                ));
                text.push(Text::raw(format!("{}\n", item[1])));
            }
            let block = Paragraph::new(text.iter())
                .block(
                    Block::default()
                        .title("Help")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::White)),
                )
                .style(Style::default())
                .alignment(Alignment::Left)
                .wrap(true);
            let area = centered_rect(50, 80, f.size());
            f.render_widget(Clear, area); //this clears out the background
            f.render_widget(block, area);
        }
    }
    pub fn render_version_selector<B: Backend>(&mut self, f: &mut Frame<B>) {
        if let Some(d) = self.get_current_dep() {
            if self.popup_shown {
                let items = self.versions.items.iter().map(|i| Text::raw(i));
                let block = List::new(items)
                    .block(
                        Block::default()
                            .title("Versions")
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::Red)),
                    )
                    .style(Style::default())
                    .highlight_style(Style::default().bg(Color::Red));

                let area = centered_rect(50, 80, f.size());
                f.render_widget(Clear, area); //this clears out the background
                f.render_widget(block, area);
            }
        }
    }

    pub fn render_tabs<B: Backend>(&mut self, mut f: &mut Frame<B>, chunk: Vec<Rect>) {
        let tabs = Tabs::default()
            .block(Block::default())
            .titles(&self.tabs.titles)
            .select(self.tabs.index)
            .style(Style::default().fg(Color::Cyan))
            .highlight_style(Style::default().fg(Color::Yellow));
        f.render_widget(tabs, chunk[0]);
        self.render_dependency_list(&mut f, chunk[1]);
        // f.render_widget(inner, chunk[1]);
    }

    pub fn render_dependency_info<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        let dep = self.get_current_dep();
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
