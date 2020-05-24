use crate::events::{StatefulList, TabsState};
use tui::backend::Backend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, BorderType, Borders, Clear, List, Paragraph, Tabs, Text};

use std::process::Command;
use tui::terminal::Frame;

use crate::parser::{Dep, DepListList, UpgradeType};

#[derive(Debug)]
pub struct InstallCandidate {
    pub name: String,
    pub version: String,
    pub kind: String,
}

pub struct App {
    kind: String,
    data: DepListList,
    items: StatefulList<String>,
    versions: StatefulList<String>,
    help_content_pos: u16,
    tabs: TabsState,
    popup_shown: bool,
    help_menu_shown: bool,
    message: Option<String>,
}

impl App {
    pub fn new(dep_list_list: DepListList, kind: &str) -> App {
        let dep_kinds = dep_list_list.get_dep_kinds();
        let dep_names = dep_list_list.get_dep_names_of_kind(&dep_kinds[0]);
        let mut dep_versions = vec![];
        if let Some(dep) = dep_list_list.get_dep(&dep_names[0]) {
            dep_versions = dep.get_version_strings();
        }
        App {
            kind: kind.to_string(),
            data: dep_list_list,
            items: StatefulList::with_items(dep_names),
            versions: StatefulList::with_items(dep_versions),
            help_content_pos: 0,
            tabs: TabsState::new(dep_kinds),
            popup_shown: false,
            help_menu_shown: false,
            message: None
        }
    }

    pub fn get_current_dep(&mut self) -> Option<Dep> {
        self.data.get_dep(&self.items.get_item())
    }

    pub fn get_selected_version(&mut self) -> String {
        self.versions.get_item()
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

    pub fn open_package_repo(&mut self) {
        let dep = self.data.get_dep(&self.items.get_item());
        if let Some(de) = dep {
            Command::new("open")
                .arg(de.get_package_repo())
                .output()
                .expect("Failed to execute command");
        }
    }

    pub fn hide_popup(&mut self) {
        self.popup_shown = false;
        self.help_menu_shown = false;
    }
    pub fn toggle_popup(&mut self) {
        if self.popup_shown {
            self.popup_shown = false
        } else if !self.help_menu_shown {
            self.popup_shown = true
        }
    }
    pub fn toggle_help_menu(&mut self) {
        if self.help_menu_shown {
            self.help_menu_shown = false
        } else if !self.popup_shown {
            self.help_menu_shown = true
        }
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
        } else if self.help_menu_shown {
            self.help_content_pos += 1;
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
        } else if self.help_menu_shown {
            if self.help_content_pos > 0 {
                self.help_content_pos -= 1;
            }
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

    pub fn get_install_candidate(&mut self) -> Option<InstallCandidate> {
        if self.popup_shown {
            let current_dep = self.get_current_dep().unwrap();
            let version_string = self.get_selected_version();
            return Some(InstallCandidate {
                name: current_dep.name,
                version: version_string,
                kind: current_dep.kind,
            });
        }
        None
    }

    pub fn set_message(&mut self, message: &str) {
        self.message = Some(message.to_string());
    }

    pub fn remove_message(&mut self) {
        self.message = None;
    }

    pub fn display_message<B: Backend>(&mut self, f: &mut Frame<B>) {
        if let Some(message) = &self.message {
            self.popup_shown = false;  // remove that version popup
            let text = vec![
                Text::raw(message)
            ];
            let block = Paragraph::new(text.iter())
                .block(
                    Block::default()
                        .title("Message")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::White)),
                )
                .style(Style::default())
                .alignment(Alignment::Left)
                .scroll(self.help_content_pos)
                .wrap(true);
            let area = centered_rect_absolute(50, 3 ,f.size());
            f.render_widget(Clear, area); //this clears out the background
            f.render_widget(block, area);
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
                ["p", "open package repo"],
            ];
            let mut text = vec![];
            text.push(Text::styled(
                "Keybindings\n",
                Style::default()
                    .fg(Color::Cyan)
                    .modifier(Modifier::UNDERLINED),
            ));
            for item in help_items.iter() {
                text.push(Text::styled(
                    format!("{:<10}", item[0]),
                    Style::default().fg(Color::Green),
                ));
                text.push(Text::raw(format!("{}\n", item[1])));
            }
            text.push(Text::styled(
                "\nColors\n",
                Style::default()
                    .fg(Color::Cyan)
                    .modifier(Modifier::UNDERLINED),
            ));
            text.push(Text::styled("Green", Style::default().fg(Color::Green)));
            text.push(Text::styled(" Patch upgrade\n", Style::default()));
            text.push(Text::styled("Magenta", Style::default().fg(Color::Magenta)));
            text.push(Text::styled(" Minor upgrade\n", Style::default()));
            text.push(Text::styled("Red", Style::default().fg(Color::Red)));
            text.push(Text::styled(" Major upgrade\n", Style::default()));
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
                .scroll(self.help_content_pos)
                .wrap(true);
            let area = centered_rect(50, 50, f.size());
            f.render_widget(Clear, area); //this clears out the background
            f.render_widget(block, area);
        }
    }
    pub fn render_version_selector<B: Backend>(&mut self, f: &mut Frame<B>) {
        if let Some(d) = self.get_current_dep() {
            // let upgrade_type = d.get_ugrade_type();
            if self.popup_shown {
                let mut items = vec![];
                for item in self.versions.items.iter() {
                    if &d.current_version.to_string() == item
                        && &d.get_latest_semver_version() == item
                    {
                        items.push(Text::styled(
                            format!("{} current&latest-semver", item),
                            Style::default().fg(Color::Cyan),
                        ));
                    } else if &d.current_version.to_string() == item {
                        items.push(Text::styled(
                            format!("{} current", item),
                            Style::default().fg(Color::Cyan),
                        ));
                    } else if &d.get_latest_semver_version() == item {
                        items.push(Text::styled(
                            format!("{} latest-semver", item),
                            Style::default().fg(Color::Green),
                        ));
                    } else {
                        items.push(Text::raw(item));
                    }
                }

                let mut color = Color::White;
                let current_item = self.versions.state.selected();
                if let Some(ci) = current_item {
                    let item = &self.versions.items[ci];
                    if &d.current_version.to_string() == item {
                        color = Color::Cyan;
                    } else if &d.get_latest_semver_version() == item {
                        color = Color::Green;
                    }
                }

                let block = List::new(items.into_iter())
                    .block(
                        Block::default()
                            .title("Versions")
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::Red)),
                    )
                    .style(Style::default())
                    .highlight_style(Style::default().fg(color))
                    .highlight_symbol("■ "); // ║ ▓ ■

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
                Text::styled("Package repo:", Style::default().fg(Color::Magenta)),
                Text::raw(format!(" {}\n", &d.get_package_repo())),
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
                .highlight_symbol("■ "); // ║ ▓ ■
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

pub fn centered_rect_absolute(x: u16, y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length((r.height - y) / 2),
                Constraint::Length(y),
                Constraint::Length((r.height - y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Length((r.width - x) / 2),
                Constraint::Length(x),
                Constraint::Length((r.width - x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
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
