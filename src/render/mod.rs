use crate::events::{StatefulList, TabsState};
use tui::backend::Backend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, BorderType, Borders, Clear, List, Paragraph, Tabs, Text};

use std::process::Command;
use tui::terminal::Frame;

use crate::parser::{Dep, DepListList, UpgradeType};

pub struct App {
    data: DepListList,
    items: StatefulList<String>,
    versions: StatefulList<String>,
    tabs: TabsState,
    popup_shown: bool,
    help_menu_shown: bool,
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
        }
    }

    pub fn get_current_dep(&mut self) -> Option<Dep> {
        self.data.get_dep(&self.items.get_item())
    }

    pub fn open_homepage(&mut self) {
        let dep = self.data.get_dep(&self.items.get_item());
        if let Some(d) = dep {
            let homepage = d.homepage;

            match homepage {
                Some(hp) => {
                    Command::new("open")
                        .arg(hp)
                        .output()
                        .expect("Failed to execute command");
                }
                None => {}
            }
        }
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

    pub fn top(&mut self) {
        if self.popup_shown {
            self.versions.first();
            self.versions.next()
        } else {
            self.items.first();
            let mut dep_versions = vec![];
            if let Some(dep) = self.get_current_dep() {
                dep_versions = dep.get_version_strings();
            }
            self.versions = StatefulList::with_items(dep_versions);
            self.versions.next();
        }
    }

    pub fn bottom(&mut self) {
        if self.popup_shown {
            self.versions.last();
        } else {
            self.items.last();
            let mut dep_versions = vec![];
            if let Some(dep) = self.get_current_dep() {
                dep_versions = dep.get_version_strings();
            }
            self.versions = StatefulList::with_items(dep_versions);
            self.versions.next();
        }
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
            let area = centered_rect(50, 50, f.size());
            f.render_widget(Clear, area); //this clears out the background
            f.render_widget(block, area);
        }
    }
    pub fn render_version_selector<B: Backend>(&mut self, f: &mut Frame<B>) {
        if let Some(d) = self.get_current_dep() {
            // TODO
            let upgrade_type = d.get_ugrade_type();
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
                    .highlight_style(Style::default().bg(Color::White));

                let area = centered_rect(50, 50, f.size());
                f.render_widget(Clear, area); //this clears out the background
                f.render_stateful_widget(block, area, &mut self.versions.state);
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
                Text::raw(format!(" {}\n", &d.get_author())),
                Text::styled("Homepage", Style::default().fg(Color::Magenta)),
                Text::raw(format!(" {}\n", &d.get_homepage())),
                Text::styled("License", Style::default().fg(Color::Yellow)),
                Text::raw(format!(" {}\n", &d.get_license())),
                Text::styled("Description", Style::default().fg(Color::Cyan)),
                Text::raw(format!(" {}\n", &d.get_description())),
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
        // let items = self.items.items.iter().map(|i| Text::raw(i));
        if let Some(dc) = self.get_current_dep() {
            let dc_upgrade_type = dc.get_ugrade_type();
            let mut items = vec![];
            for item in self.items.items.iter() {
                let dep = self.data.get_dep(item);
                match dep {
                    Some(d) => {
                        let upgrade_type = d.get_ugrade_type();
                        items.push(Text::styled(
                            format!(
                                "{} ({} > {})",
                                d.name,
                                d.current_version.to_string(),
                                d.get_latest_semver_version()
                            ),
                            Style::default().fg(get_version_color(upgrade_type)),
                        ));
                    }
                    None => unreachable!(),
                }
            }
            let block = List::new(items.into_iter())
                .block(
                    Block::default()
                        .title("Dependencies")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::White)),
                )
                .style(Style::default())
                .highlight_style(Style::default().fg(get_version_color(dc_upgrade_type)))
                .highlight_symbol("■ ");  // ║ ▓ ■
            f.render_stateful_widget(block, chunk, &mut self.items.state);
        }
    }
}

fn get_version_color(upgrage_type: UpgradeType) -> Color {
    match upgrage_type {
        UpgradeType::None => Color::White,
        UpgradeType::Major => Color::Red,
        UpgradeType::Minor => Color::Magenta,
        UpgradeType::Patch => Color::Green,
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
