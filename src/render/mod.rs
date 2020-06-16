use crate::events::{StatefulList, TabsState};
use termion::event::Key;
use tui::backend::Backend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, BorderType, Borders, Clear, List, Paragraph, Tabs, Text};

use std::process::Command;
use tui::terminal::Frame;

use crate::parser::determinekind::ParserKind;
use crate::parser::{stringify, Project, UpgradeType};

pub struct AppState {
    tab: usize,
    dep: Option<usize>,
}

#[derive(Debug)]
pub struct InstallCandidate {
    pub name: String,
    pub version: String,
    pub kind: String,
}

#[derive(Debug)]
pub enum PopupKind {
    Message,
    Help,
    Versions,
    SearchInput,
    SearchList,
    None,
}
pub struct App {
    folder: String, // TODO: Change this to path
    kind: ParserKind,
    project: Project,
    tabs: TabsState,
    items: StatefulList<String>,
    versions: StatefulList<String>,
    pub popup: PopupKind,
    help_content_pos: u16,
    message: Option<String>,
    pub search_string: String,
    pub search_result: StatefulList<String>,
}

impl App {
    pub fn new(project: Project, kind: ParserKind, folder: &str) -> App {
        let dep_kinds = project.get_groups();
        let dep_names = project.get_deps_in_group(&dep_kinds[0]);
        let mut dep_versions = vec![];
        if dep_names.len() > 0 {
            if let Some(dep) = project.get_dep_versions(&dep_names[0]) {
                dep_versions = dep.into_iter().map(|x| x.to_string()).collect();
            }
        }
        App {
            folder: folder.to_string(),
            kind,
            project,
            tabs: TabsState::new(dep_kinds),
            items: StatefulList::with_items(dep_names),
            versions: StatefulList::with_items(dep_versions),
            message: None,
            popup: PopupKind::None,
            help_content_pos: 0,
            search_result: StatefulList::with_items(vec![]),
            search_string: "".to_string(),
        }
    }

    fn get_current_version_strings(&self) -> Vec<String> {
        let current_dep = self.get_current_dep_name();
        match current_dep {
            Some(dep) => match self.project.get_dep_versions(&dep) {
                Some(v) => v.clone().into_iter().map(|x| x.to_string()).collect(),
                None => vec![],
            },
            _ => vec![],
        }
    }

    pub fn get_selected_version(&mut self) -> String {
        self.versions.get_item()
    }

    pub fn open_homepage(&mut self) {
        let current_dep = self.get_current_dep_name();
        if let Some(dep) = current_dep {
            let homepage = self.project.get_homepage(&dep);
            if let Some(hp) = homepage {
                Command::new("open")
                    .arg(hp)
                    .output()
                    .expect("Failed to execute command");
            }
        }
    }

    pub fn open_repository(&mut self) {
        let current_dep = self.get_current_dep_name();
        if let Some(dep) = current_dep {
            let repository = self.project.get_repository(&dep);
            if let Some(rp) = repository {
                Command::new("open")
                    .arg(rp)
                    .output()
                    .expect("Failed to execute command");
            }
        }
    }

    pub fn unwrap_popup(&mut self) {
        self.popup = match self.popup {
            PopupKind::SearchList => PopupKind::SearchInput,
            PopupKind::SearchInput => PopupKind::None,
            PopupKind::Help => PopupKind::None,
            PopupKind::Versions => PopupKind::None,
            PopupKind::Message => {
                if self.search_string.len() > 0 {
                    PopupKind::SearchInput
                } else {
                    PopupKind::None
                }
            }
            PopupKind::None => PopupKind::None,
        };
    }
    pub fn toggle_versions_menu(&mut self) {
        if let PopupKind::None = self.popup {
            if !self
                .project
                .is_versions_available(&self.get_current_dep_name().unwrap())
            {
                self.message = Some("No versions available".to_string());
                self.popup = PopupKind::Message;
                return;
            }
            self.popup = PopupKind::Versions;
        }
    }
    pub fn toggle_help_menu(&mut self) {
        match self.popup {
            PopupKind::None => {
                self.popup = PopupKind::Help;
            }
            PopupKind::Help => {
                self.popup = PopupKind::None;
            }
            _ => {}
        }
    }

    pub fn tab_next(&mut self) {
        self.tabs.next();
        let dep_names = self
            .project
            .get_deps_in_group(&self.tabs.titles[self.tabs.index]);
        self.items = StatefulList::with_items(dep_names);
        self.items.next();
        let dep_versions = self.get_current_version_strings();
        self.versions = StatefulList::with_items(dep_versions);
        self.versions.state.select(self.get_current_version_index());
    }
    pub fn tab_previous(&mut self) {
        self.tabs.previous();
        let dep_names = self
            .project
            .get_deps_in_group(&self.tabs.titles[self.tabs.index]);
        self.items = StatefulList::with_items(dep_names);
        self.items.next();

        let dep_versions = self.get_current_version_strings();
        self.versions = StatefulList::with_items(dep_versions);
        self.versions.state.select(self.get_current_version_index());
    }

    pub fn _get_current_tab_name(&self) -> String {
        self.tabs.titles[self.tabs.index].to_string()
    }

    pub fn top(&mut self) {
        if let PopupKind::Versions = self.popup {
            self.versions.first();
            self.versions.next()
        } else {
            self.items.first();
            let dep_versions = self.get_current_version_strings();
            self.versions = StatefulList::with_items(dep_versions);
            self.versions.state.select(self.get_current_version_index());
        }
    }

    pub fn bottom(&mut self) {
        if let PopupKind::Versions = self.popup {
            self.versions.last();
        } else {
            self.items.last();
            let dep_versions = self.get_current_version_strings();
            self.versions = StatefulList::with_items(dep_versions);
            self.versions.state.select(self.get_current_version_index());
        }
    }

    pub fn next(&mut self) {
        match self.popup {
            PopupKind::Versions => self.versions.next(),
            PopupKind::Help => self.help_content_pos += 1,
            PopupKind::SearchList => self.search_result.next(),
            _ => {
                self.items.next();
                let dep_versions = self.get_current_version_strings();
                self.versions = StatefulList::with_items(dep_versions);
                self.versions.state.select(self.get_current_version_index());
            }
        }
    }

    pub fn previous(&mut self) {
        match self.popup {
            PopupKind::Versions => self.versions.previous(),
            PopupKind::Help => {
                if self.help_content_pos > 0 {
                    self.help_content_pos -= 1;
                }
            }
            PopupKind::SearchList => self.search_result.previous(),
            _ => {
                self.items.previous();
                let dep_versions = self.get_current_version_strings();
                self.versions = StatefulList::with_items(dep_versions);
                self.versions.state.select(self.get_current_version_index());
            }
        }
    }

    pub fn get_state(&self) -> AppState {
        let dep = match self.items.state.selected() {
            Some(m) => match m {
                0 => {
                    if self.items.items.len() == 0 {
                        None
                    } else {
                        Some(0)
                    }
                }
                m => Some(m - 1),
            },
            None => None,
        };
        AppState {
            tab: self.tabs.index,
            dep,
        }
    }

    pub fn set_state(&mut self, state: AppState) {
        self.tabs.index = state.tab;
        let dep_names = self
            .project
            .get_deps_in_group(&self.tabs.titles[self.tabs.index]);
        self.items = StatefulList::with_items(dep_names);
        self.items.state.select(state.dep);
        let dep_versions = self.get_current_version_strings();
        self.versions = StatefulList::with_items(dep_versions);
        self.versions.state.select(self.get_current_version_index());
    }

    pub fn delete_current_dep(&self) -> bool {
        let current_tab = &self.get_current_group_name().unwrap();
        let current_dep = self.get_current_dep_name();
        if let Some(cd) = &current_dep {
            self.project.delete_dep(&self.kind, &self.folder, &current_tab, &cd)
        } else {
            false
        }
    }

    // pub fn get_install_candidate(&mut self) -> Option<InstallCandidate> {
    //     match self.popup {
    //         PopupKind::Versions => {
    //             let current_dep = self.get_current_dep_name().unwrap();
    //             let version_string = self.get_selected_version();
    //             Some(InstallCandidate {
    //                 name: current_dep,
    //                 version: version_string,
    //                 kind: self.tabs.titles[self.tabs.index].to_string(),
    //             })
    //         }
    //         PopupKind::SearchList => {
    //             let search_dep =
    //                 self.search_result.items[self.search_result.state.selected().unwrap()].clone();
    //             Some(InstallCandidate {
    //                 name: search_dep,  // TODO
    //                 version: search_dep,
    //                 kind: self.tabs.titles[self.tabs.index].to_string(),
    //             })
    //         }
    //         _ => None,
    //     }
    // }

    pub fn set_message(&mut self, message: &str) {
        self.message = Some(message.to_string());
        self.popup = PopupKind::Message;
    }

    pub fn remove_message(&mut self) {
        self.message = None;
    }

    fn get_current_dep_name(&self) -> Option<String> {
        if &self.tabs.index < &self.tabs.titles.len() {
            let group = &self.tabs.titles[self.tabs.index];
            match &self.items.state.selected() {
                Some(ds) => Some(self.items.get_item()),
                None => None,
            }
        } else {
            None
        }
    }

    // pub fn show_searches(&mut self, r: Vec<SearchDep>) {
    //     self.popup = PopupKind::SearchList;
    //     self.search_result = StatefulList::with_items(r);
    //     self.search_result.next();
    // }

    pub fn search_update(&mut self, input: Key) {
        match input {
            Key::Char(s) => {
                self.search_string.push(s);
            }
            Key::Backspace => {
                self.search_string.pop();
            }
            _ => unreachable!(),
        }
    }

    pub fn display_search_input<B: Backend>(&mut self, f: &mut Frame<B>) {
        if let PopupKind::SearchInput = self.popup {
            let text = vec![Text::raw(&self.search_string)];
            let block = Paragraph::new(text.iter())
                .block(
                    Block::default()
                        .title("Search")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::White)),
                )
                .style(Style::default())
                .alignment(Alignment::Left)
                .scroll(self.help_content_pos)
                .wrap(true);
            let area = centered_rect_absolute(50, 3, f.size());
            f.render_widget(Clear, area); //this clears out the background
            f.render_widget(block, area);
        }
    }

    pub fn display_message<B: Backend>(&mut self, f: &mut Frame<B>) {
        if let PopupKind::Message = self.popup {
            if let Some(message) = &self.message {
                let text = vec![Text::raw(message)];
                let block = Paragraph::new(text.iter())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::White)),
                    )
                    .style(Style::default())
                    .alignment(Alignment::Left)
                    .scroll(self.help_content_pos)
                    .wrap(true);
                let area = centered_rect_absolute(50, 3, f.size());
                f.render_widget(Clear, area); //this clears out the background
                f.render_widget(block, area);
            }
        }
    }

    pub fn render_help_menu<B: Backend>(&mut self, f: &mut Frame<B>) {
        if let PopupKind::Help = self.popup {
            let help_items = [
                ["?", "show help menu"],
                ["j/down", "move down"],
                ["k/up", "move up"],
                ["h/left", "prev tab"],
                ["l/right", "next tab"],
                ["v/space", "show version list"],
                ["o", "open homepage"],
                ["p", "open package repo"],
                ["s", "search for package"],
                ["D", "delete package"],
                ["enter", "update/install package"],
                ["q", "quit depman"],
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

    pub fn render_search_results<B: Backend>(&mut self, f: &mut Frame<B>) {
        if let PopupKind::SearchList = self.popup {
            let mut results = vec![];
            for item in &self.search_result.items {
                results.push(Text::raw(format!(
                    "{} {}",
                    item,
                    &stringify(&self.project.get_current_version(item))
                )))
            }
            let block = List::new(results.into_iter())
                .block(
                    Block::default()
                        .title("Search result")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Red)),
                )
                .style(Style::default())
                .highlight_style(Style::default())
                .highlight_symbol("■ "); // ║ ▓ ■

            let area = centered_rect(80, 50, f.size());
            f.render_widget(Clear, area); //this clears out the background
            f.render_stateful_widget(block, area, &mut self.search_result.state);
        }
    }

    pub fn render_version_selector<B: Backend>(&mut self, f: &mut Frame<B>) {
        let current_tab = &self.get_current_group_name().unwrap();
        if let Some(d) = self.get_current_dep_name() {
            if let PopupKind::Versions = self.popup {
                let mut items = vec![];
                for item in self.versions.items.iter() {
                    if &stringify(&self.project.get_current_version(&d)) == item
                        && &stringify(&self.project.get_semver_version(&current_tab, &d)) == item
                    {
                        items.push(Text::styled(
                            format!("{} current&latest-semver", item),
                            Style::default().fg(Color::Cyan),
                        ));
                    } else if &stringify(&self.project.get_current_version(&d)) == item {
                        items.push(Text::styled(
                            format!("{} current", item),
                            Style::default().fg(Color::Cyan),
                        ));
                    } else if &stringify(&self.project.get_semver_version(&current_tab, &d)) == item
                    {
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
                    if &stringify(&self.project.get_current_version(&d)) == item {
                        color = Color::Cyan;
                    } else if &stringify(&self.project.get_semver_version(&current_tab, &d)) == item
                    {
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

    pub fn get_current_version_index(&self) -> Option<usize> {
        if let Some(d) = &self.get_current_dep_name() {
            for (i, item) in self.versions.items.iter().enumerate() {
                if &stringify(&self.project.get_current_version(&d)) == item {
                    return Some(i);
                }
            }
        }
        None
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
        let current_tab = &self.get_current_group_name().unwrap();
        let dep = self.get_current_dep_name();
        if let Some(d) = dep {
            let text = [
                Text::styled("Name", Style::default().fg(Color::Red)),
                Text::raw(format!(" {}\n", d)),
                Text::styled("Specified Version", Style::default().fg(Color::Blue)),
                Text::raw(format!(
                    " {}\n",
                    stringify(&self.project.get_specified_version(&current_tab, &d))
                )),
                Text::styled("Current Version", Style::default().fg(Color::Blue)),
                Text::raw(format!(
                    " {}\n",
                    stringify(&self.project.get_current_version(&d))
                )),
                Text::styled("Upgradeable Version", Style::default().fg(Color::Blue)),
                Text::raw(format!(
                    " {}\n",
                    stringify(&self.project.get_semver_version(&current_tab, &d))
                )),
                Text::styled("Latest Version", Style::default().fg(Color::Blue)),
                Text::raw(format!(
                    " {}\n",
                    stringify(&self.project.get_latest_version(&d))
                )),
                Text::styled("Author", Style::default().fg(Color::Green)),
                Text::raw(format!(" {}\n", stringify(&self.project.get_author(&d)))),
                Text::styled("Homepage", Style::default().fg(Color::Magenta)),
                Text::raw(format!(" {}\n", stringify(&self.project.get_homepage(&d)))),
                Text::styled("Package repo:", Style::default().fg(Color::Magenta)),
                Text::raw(format!(
                    " {}\n",
                    stringify(&self.project.get_repository(&d))
                )),
                Text::styled("License", Style::default().fg(Color::Yellow)),
                Text::raw(format!(" {}\n", stringify(&self.project.get_license(&d)))),
                Text::styled("Description", Style::default().fg(Color::Cyan)),
                Text::raw(format!(
                    " {}\n",
                    stringify(&self.project.get_description(&d))
                )),
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

    fn get_current_group_name(&self) -> Option<String> {
        if &self.tabs.index < &self.tabs.titles.len() {
            Some(self.tabs.titles[self.tabs.index].to_string())
        } else {
            None
        }
    }

    pub fn render_dependency_list<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        if let Some(dc) = self.get_current_dep_name() {
            let current_tab = &self.get_current_group_name().unwrap();
            let dc_upgrade_type = self.project.get_upgrade_type(&current_tab, &dc);
            let mut items = vec![];
            for item in self.items.items.iter() {
                let upgrade_type = self.project.get_upgrade_type(&current_tab, &item);
                // use UpgradeType::Breaking instead of is_newer_available
                let breaking_changes_string = match upgrade_type {
                    UpgradeType::Breaking => "+",
                    _ => "",
                };
                items.push(Text::styled(
                    format!(
                        "{} ({} > {})  {}",
                        item,
                        stringify(&self.project.get_current_version(&item)),
                        stringify(&self.project.get_semver_version(&current_tab, &item)),
                        breaking_changes_string
                    ),
                    Style::default().fg(get_version_color(upgrade_type)),
                ));
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
        } else {
            let text = vec![Text::styled(
                "No dependencies available",
                Style::default().fg(Color::White),
            )];
            let block = Paragraph::new(text.iter())
                .block(
                    Block::default()
                        .title("Dependencies")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::White)),
                )
                .style(Style::default())
                .alignment(Alignment::Center)
                .scroll(self.help_content_pos)
                .wrap(true);
            f.render_widget(block, chunk);
        }
    }
}

fn get_version_color(upgrage_type: UpgradeType) -> Color {
    match upgrage_type {
        UpgradeType::None => Color::White,
        UpgradeType::Major => Color::Red,
        UpgradeType::Minor => Color::Magenta,
        UpgradeType::Patch => Color::Green,
        UpgradeType::Breaking => Color::White,
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
